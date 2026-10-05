#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{OnceLock, RwLock};

use op_paint::{Color, DisplayList, PaintCommand};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, EndPaint, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    FillRect, GetStockObject, IntersectClipRect, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    SelectObject, SetBkMode, SetTextColor, SetViewportOrgEx, TRANSPARENT, TextOutW, UpdateWindow,
    WHITE_BRUSH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, SetFocus, VK_CONTROL, VK_F5, VK_L, VK_RETURN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static DISPLAY_LIST: OnceLock<RwLock<DisplayList>> = OnceLock::new();
static PAINTED_ONCE: AtomicBool = AtomicBool::new(false);
static SCROLL_Y: AtomicI32 = AtomicI32::new(0);
const TOOLBAR_HEIGHT: i32 = 76;
const BACK: i32 = 101;
const FORWARD: i32 = 102;
const RELOAD: i32 = 103;
const ADDRESS: i32 = 104;
const GO: i32 = 105;
const STATUS: i32 = 106;
const POLL_TIMER: usize = 1;
const NAVIGATION_COMMAND: u32 = WM_APP + 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationEvent {
    Navigate(String),
    Back,
    Forward,
    Reload,
    Poll,
}

pub struct NativeBrowserWindow {
    hwnd: HWND,
}

impl NativeBrowserWindow {
    pub fn create(title: &str, display_list: DisplayList) -> Result<Self, String> {
        PAINTED_ONCE.store(false, Ordering::SeqCst);
        SCROLL_Y.store(0, Ordering::SeqCst);
        set_display_list(display_list);

        let class_name = wide("OPBrowserMainWindow");
        let window_title = wide(title);

        let hwnd = unsafe {
            let instance = GetModuleHandleW(null());
            if instance.is_null() {
                return Err("GetModuleHandleW failed".into());
            }

            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: GetStockObject(WHITE_BRUSH),
                lpszMenuName: null(),
                lpszClassName: class_name.as_ptr(),
            };

            if RegisterClassW(&class) == 0 {
                return Err("RegisterClassW failed".into());
            }

            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1280,
                800,
                null_mut(),
                null_mut(),
                instance,
                null::<c_void>(),
            )
        };

        if hwnd.is_null() {
            return Err("CreateWindowExW failed".into());
        }

        let window = Self { hwnd };
        for (id, class, text, style) in [
            (BACK, "BUTTON", "Back", 0),
            (FORWARD, "BUTTON", "Forward", 0),
            (RELOAD, "BUTTON", "Reload", 0),
            (ADDRESS, "EDIT", "", WS_BORDER | ES_AUTOHSCROLL as u32),
            (GO, "BUTTON", "Go", 0),
            (
                STATUS,
                "STATIC",
                "Ready - enter an http:// or https:// address",
                0,
            ),
        ] {
            let control = unsafe {
                CreateWindowExW(
                    0,
                    wide(class).as_ptr(),
                    wide(text).as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
                    0,
                    0,
                    1,
                    1,
                    hwnd,
                    id as usize as HMENU,
                    GetModuleHandleW(null()),
                    null(),
                )
            };
            if control.is_null() {
                unsafe {
                    DestroyWindow(hwnd);
                }
                return Err("failed to create navigation control".into());
            }
        }
        unsafe {
            SendMessageW(
                GetDlgItem(hwnd, ADDRESS),
                0x00C5, /* EM_LIMITTEXT */
                16_384,
                0,
            );
        }
        layout_controls(hwnd);
        window.set_navigation_state(false, false, false, false);
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
        }

        Ok(window)
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn painted_once(&self) -> bool {
        PAINTED_ONCE.load(Ordering::SeqCst)
    }

    pub fn viewport_size(&self) -> (i32, i32) {
        let mut rect: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }
        (rect.right.max(240), (rect.bottom - TOOLBAR_HEIGHT).max(100))
    }

    pub fn set_address(&self, address: &str) {
        unsafe {
            SetWindowTextW(GetDlgItem(self.hwnd, ADDRESS), wide(address).as_ptr());
        }
    }

    pub fn address(&self) -> String {
        let control = unsafe { GetDlgItem(self.hwnd, ADDRESS) };
        let length = unsafe { GetWindowTextLengthW(control) }.max(0) as usize;
        let mut buffer = vec![0u16; length + 1];
        let read = unsafe { GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32) };
        String::from_utf16_lossy(&buffer[..read.max(0) as usize])
    }

    pub fn set_status(&self, message: &str) {
        unsafe {
            SetWindowTextW(GetDlgItem(self.hwnd, STATUS), wide(message).as_ptr());
        }
    }

    pub fn set_navigation_state(&self, back: bool, forward: bool, reload: bool, busy: bool) {
        unsafe {
            for (id, enabled) in [
                (BACK, back && !busy),
                (FORWARD, forward && !busy),
                (RELOAD, reload && !busy),
                (GO, !busy),
            ] {
                EnableWindow(GetDlgItem(self.hwnd, id), i32::from(enabled));
            }
            if busy {
                SetTimer(self.hwnd, POLL_TIMER, 30, None);
            } else {
                KillTimer(self.hwnd, POLL_TIMER);
            }
        }
    }

    pub fn present(&self, address: &str, display_list: DisplayList) {
        self.set_address(address);
        set_display_list(display_list);
        SCROLL_Y.store(0, Ordering::SeqCst);
        PAINTED_ONCE.store(false, Ordering::SeqCst);
        unsafe {
            SetWindowTextW(self.hwnd, wide(&format!("OPBrowser - {address}")).as_ptr());
            InvalidateRect(self.hwnd, null(), 1);
            UpdateWindow(self.hwnd);
        }
    }

    /// Exercises the same queued Enter-key path as a user pasting an address.
    pub fn submit_address(&self, address: &str) {
        self.set_address(address);
        unsafe {
            PostMessageW(
                GetDlgItem(self.hwnd, ADDRESS),
                WM_KEYDOWN,
                VK_RETURN as usize,
                0,
            );
        }
    }

    pub fn close(&self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }

    pub fn run_message_loop(&self, mut on_event: impl FnMut(NavigationEvent)) -> i32 {
        let mut message: MSG = unsafe { zeroed() };

        loop {
            let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
            if result == -1 {
                return 1;
            }
            if result == 0 {
                return message.wParam as i32;
            }

            if let Some(event) = self.navigation_event(&message) {
                on_event(event);
                continue;
            }
            if message.message == WM_KEYDOWN
                && message.wParam == VK_L as usize
                && unsafe { GetKeyState(VK_CONTROL as i32) } < 0
            {
                unsafe {
                    let address = GetDlgItem(self.hwnd, ADDRESS);
                    SetFocus(address);
                    SendMessageW(address, 0x00B1 /* EM_SETSEL */, 0, -1);
                }
                continue;
            }

            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn navigation_event(&self, message: &MSG) -> Option<NavigationEvent> {
        match message.message {
            WM_KEYDOWN
                if message.wParam == VK_RETURN as usize
                    && message.hwnd == unsafe { GetDlgItem(self.hwnd, ADDRESS) } =>
            {
                Some(NavigationEvent::Navigate(self.address()))
            }
            WM_KEYDOWN if message.wParam == VK_F5 as usize => Some(NavigationEvent::Reload),
            NAVIGATION_COMMAND if message.hwnd == self.hwnd => match message.wParam as i32 {
                BACK => Some(NavigationEvent::Back),
                FORWARD => Some(NavigationEvent::Forward),
                RELOAD => Some(NavigationEvent::Reload),
                GO => Some(NavigationEvent::Navigate(self.address())),
                _ => None,
            },
            WM_TIMER if message.hwnd == self.hwnd && message.wParam == POLL_TIMER => {
                Some(NavigationEvent::Poll)
            }
            _ => None,
        }
    }
}

impl Drop for NativeBrowserWindow {
    fn drop(&mut self) {
        if unsafe { IsWindow(self.hwnd) } != 0 {
            self.close();
        }
    }
}

fn layout_controls(hwnd: HWND) {
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
        for (id, x, y, width, height) in [
            (BACK, 8, 8, 60, 28),
            (FORWARD, 72, 8, 70, 28),
            (RELOAD, 146, 8, 70, 28),
            (ADDRESS, 224, 8, (rect.right - 296).max(20), 28),
            (GO, (rect.right - 64).max(250), 8, 56, 28),
            (STATUS, 8, 44, (rect.right - 16).max(20), 24),
        ] {
            MoveWindow(GetDlgItem(hwnd, id), x, y, width, height, 1);
        }
    }
}

fn set_display_list(display_list: DisplayList) {
    let storage = DISPLAY_LIST.get_or_init(|| RwLock::new(DisplayList::default()));

    if let Ok(mut current) = storage.write() {
        *current = display_list;
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COMMAND
            if wparam >> 16 == 0
                && matches!((wparam & 0xffff) as i32, BACK | FORWARD | RELOAD | GO) =>
        {
            unsafe {
                PostMessageW(hwnd, NAVIGATION_COMMAND, wparam & 0xffff, 0);
            }
            0
        }
        WM_SIZE => {
            layout_controls(hwnd);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as u16 as i16) as i32;
            let mut rect: RECT = unsafe { zeroed() };
            unsafe {
                GetClientRect(hwnd, &mut rect);
            }
            let height = DISPLAY_LIST
                .get()
                .and_then(|storage| storage.read().ok())
                .map(|list| {
                    list.commands
                        .iter()
                        .filter_map(|command| match command {
                            PaintCommand::FillRect { y, height, .. } => Some(y + height),
                            _ => None,
                        })
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            let max_scroll = (height - (rect.bottom - TOOLBAR_HEIGHT)).max(0);
            let old = SCROLL_Y.load(Ordering::SeqCst);
            SCROLL_Y.store(
                (old - delta / 120 * 72).clamp(0, max_scroll),
                Ordering::SeqCst,
            );
            unsafe {
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_PAINT => {
            paint_window(hwnd);
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn paint_window(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = unsafe { zeroed() };
    let hdc = unsafe { BeginPaint(hwnd, &mut paint) };

    if hdc.is_null() {
        return;
    }

    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
        IntersectClipRect(hdc, 0, TOOLBAR_HEIGHT, rect.right, rect.bottom);
        SetViewportOrgEx(
            hdc,
            0,
            TOOLBAR_HEIGHT - SCROLL_Y.load(Ordering::SeqCst),
            null_mut(),
        );
    }

    if let Some(storage) = DISPLAY_LIST.get()
        && let Ok(display_list) = storage.read()
    {
        for command in &display_list.commands {
            paint_command(hdc, command);
        }
    }

    unsafe {
        EndPaint(hwnd, &paint);
    }

    PAINTED_ONCE.store(true, Ordering::SeqCst);
}

fn paint_command(hdc: *mut c_void, command: &PaintCommand) {
    match command {
        PaintCommand::FillRect {
            x,
            y,
            width,
            height,
            color,
        } => {
            let rect = RECT {
                left: *x,
                top: *y,
                right: *x + *width,
                bottom: *y + *height,
            };
            let brush = unsafe { CreateSolidBrush(color_ref(*color)) };

            if !brush.is_null() {
                unsafe {
                    FillRect(hdc, &rect, brush);
                    DeleteObject(brush);
                }
            }
        }
        PaintCommand::Text {
            x,
            y,
            text,
            font_size,
            bold,
            color,
        } => {
            let face = wide("Segoe UI");
            let weight = if *bold { FW_BOLD } else { FW_NORMAL } as i32;

            let font = unsafe {
                CreateFontW(
                    -*font_size,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    (DEFAULT_PITCH | FF_DONTCARE) as u32,
                    face.as_ptr(),
                )
            };

            unsafe {
                SetBkMode(hdc, TRANSPARENT as i32);
                SetTextColor(hdc, color_ref(*color));
            }

            let previous_font = if font.is_null() {
                null_mut()
            } else {
                unsafe { SelectObject(hdc, font) }
            };

            let wide_text: Vec<u16> = text.encode_utf16().collect();
            if !wide_text.is_empty() {
                unsafe {
                    TextOutW(
                        hdc,
                        *x,
                        *y,
                        wide_text.as_ptr(),
                        wide_text.len().min(i32::MAX as usize) as i32,
                    );
                }
            }

            if !font.is_null() {
                unsafe {
                    if !previous_font.is_null() {
                        SelectObject(hdc, previous_font);
                    }
                    DeleteObject(font);
                }
            }
        }
    }
}

fn color_ref(color: Color) -> u32 {
    u32::from(color.r) | (u32::from(color.g) << 8) | (u32::from(color.b) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_controls_dispatch_navigation_and_repaint() {
        let window =
            NativeBrowserWindow::create("OPBrowser UI test", DisplayList::default()).unwrap();
        window.set_navigation_state(true, true, true, false);
        window.submit_address("data:text/html,first");
        // A broken input path must fail instead of hanging the workspace tests.
        let hwnd = window.hwnd() as usize;
        let (done, stop) = std::sync::mpsc::channel();
        let watchdog = std::thread::spawn(move || {
            if stop
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_err()
            {
                unsafe {
                    PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0);
                }
            }
        });
        let mut step = 0;
        let exit = window.run_message_loop(|event| {
            match step {
                0 => {
                    assert_eq!(
                        event,
                        NavigationEvent::Navigate("data:text/html,first".into())
                    );
                    let replacement = DisplayList {
                        commands: vec![PaintCommand::FillRect {
                            x: 0,
                            y: 0,
                            width: 1200,
                            height: 2000,
                            color: Color::WHITE,
                        }],
                    };
                    window.present("http://example.test/", replacement.clone());
                    assert!(window.painted_once());
                    assert_eq!(*DISPLAY_LIST.get().unwrap().read().unwrap(), replacement);
                    unsafe {
                        SendMessageW(
                            window.hwnd,
                            WM_MOUSEWHEEL,
                            ((-120i16 as u16) as usize) << 16,
                            0,
                        );
                    }
                    assert!(SCROLL_Y.load(Ordering::SeqCst) > 0);
                    window.present("http://example.test/", replacement);
                    assert_eq!(SCROLL_Y.load(Ordering::SeqCst), 0);
                    window.set_address("data:text/html,second");
                    unsafe {
                        SendMessageW(GetDlgItem(window.hwnd, GO), BM_CLICK, 0, 0);
                    }
                }
                1 => {
                    assert_eq!(
                        event,
                        NavigationEvent::Navigate("data:text/html,second".into())
                    );
                    unsafe {
                        SendMessageW(GetDlgItem(window.hwnd, BACK), BM_CLICK, 0, 0);
                    }
                }
                2 => {
                    assert_eq!(event, NavigationEvent::Back);
                    unsafe {
                        SendMessageW(GetDlgItem(window.hwnd, FORWARD), BM_CLICK, 0, 0);
                    }
                }
                3 => {
                    assert_eq!(event, NavigationEvent::Forward);
                    unsafe {
                        SendMessageW(GetDlgItem(window.hwnd, RELOAD), BM_CLICK, 0, 0);
                    }
                }
                4 => {
                    assert_eq!(event, NavigationEvent::Reload);
                    window.set_navigation_state(true, true, true, true);
                }
                5 => {
                    assert_eq!(event, NavigationEvent::Poll);
                    window.set_navigation_state(true, true, true, false);
                    window.close();
                }
                _ => panic!("unexpected extra UI event"),
            }
            step += 1;
        });
        done.send(()).unwrap();
        watchdog.join().unwrap();
        assert_eq!(exit, 0);
        assert_eq!(step, 6);
    }
}
