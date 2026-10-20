#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};

use op_paint::{Color, DisplayList, PaintCommand};
mod raster;
use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, EndPaint, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    FillRect, GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, RestoreDC, SaveDC, ScreenToClient, SelectObject, SetBkMode,
    SetTextColor, SetViewportOrgEx, TRANSPARENT, TextOutW, UpdateWindow, WHITE_BRUSH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetCapture, GetKeyState, ReleaseCapture, SetCapture, SetFocus, TME_LEAVE,
    TRACKMOUSEEVENT, TrackMouseEvent, VK_CONTROL, VK_F5, VK_L, VK_RETURN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static DISPLAY_LIST: OnceLock<RwLock<DisplayList>> = OnceLock::new();
static PAINTED_ONCE: AtomicBool = AtomicBool::new(false);
static PAINTED_IMAGES: AtomicUsize = AtomicUsize::new(0);
static SCROLL_Y: AtomicI32 = AtomicI32::new(0);
static THUMB_DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);
static POINTER_PRESS_CAPTURE: AtomicBool = AtomicBool::new(false);
static PENDING_UTF16_HIGH: AtomicUsize = AtomicUsize::new(0);
static THUMB_DRAG_LAST_Y: AtomicI32 = AtomicI32::new(0);
static LINK_REGIONS: OnceLock<RwLock<Vec<LinkRegion>>> = OnceLock::new();

struct LinkRegion {
    bounds: RECT,
    href: String,
}
const TOOLBAR_HEIGHT: i32 = 76;
const BACK: i32 = 101;
const FORWARD: i32 = 102;
const RELOAD: i32 = 103;
const ADDRESS: i32 = 104;
const GO: i32 = 105;
const STATUS: i32 = 106;
const POLL_TIMER: usize = 1;
const RESIZE_TIMER: usize = 2;
const WM_MOUSELEAVE: u32 = 0x02A3;
const NAVIGATION_COMMAND: u32 = WM_APP + 1;
const CANCEL_THUMB_DRAG: u32 = WM_APP + 2;
const CANCEL_POINTER_PRESS: u32 = WM_APP + 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationEvent {
    Navigate(String),
    FollowLinkAt {
        href: String,
        x: i32,
        y: i32,
    },
    Click {
        x: i32,
        y: i32,
    },
    PointerDown {
        x: i32,
        y: i32,
    },
    PointerCancel,
    FocusNext {
        backwards: bool,
    },
    ActivateFocused,
    KeyDown {
        key: String,
        code: String,
        shift: bool,
        ctrl: bool,
        repeat: bool,
    },
    TextInput(String),
    KeyUp {
        key: String,
        code: String,
        shift: bool,
        ctrl: bool,
    },
    Hover {
        x: i32,
        y: i32,
    },
    Scroll {
        x: i32,
        y: i32,
        delta_y: i32,
    },
    HorizontalScroll {
        x: i32,
        y: i32,
        delta_x: i32,
    },
    ScrollbarDragStart {
        x: i32,
        y: i32,
    },
    ScrollbarDragMove {
        y: i32,
    },
    ScrollbarDragEnd,
    Back,
    Forward,
    Reload,
    Resize,
    Poll,
}

/// Paint a display list into a deterministic top-down 32-bit BGRA DIB without
/// creating a visible browser window. Compatibility reftests use the same GDI
/// command painter as the real Win32 window.
pub fn render_display_list_to_bgra(
    display_list: &DisplayList,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, String> {
    let surface = raster::Surface::new(null_mut(), width, height)
        .ok_or_else(|| "failed to create offscreen GDI surface".to_owned())?;
    surface.clear_white();
    let mut link_regions = Vec::new();
    paint_commands(surface.dc, &display_list.commands, &mut link_regions);
    unsafe {
        windows_sys::Win32::Graphics::Gdi::GdiFlush();
    }
    Ok(surface.pixels())
}

pub struct NativeBrowserWindow {
    hwnd: HWND,
}

impl NativeBrowserWindow {
    pub fn create(title: &str, display_list: DisplayList) -> Result<Self, String> {
        PAINTED_ONCE.store(false, Ordering::SeqCst);
        SCROLL_Y.store(0, Ordering::SeqCst);
        THUMB_DRAG_ACTIVE.store(false, Ordering::SeqCst);
        POINTER_PRESS_CAPTURE.store(false, Ordering::SeqCst);
        PENDING_UTF16_HIGH.store(0, Ordering::SeqCst);
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

            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
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
            // Creation sends WM_SIZE before the application starts its event loop.
            KillTimer(hwnd, RESIZE_TIMER);
        }

        Ok(window)
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn painted_once(&self) -> bool {
        PAINTED_ONCE.load(Ordering::SeqCst)
    }

    pub fn painted_image_count(&self) -> usize {
        PAINTED_IMAGES.load(Ordering::SeqCst)
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

    /// Existing document scroll fallback for wheel input which no nested
    /// original CSS overflow container accepted.
    pub fn scroll_document_by(&self, delta_y: i32) {
        scroll_document_delta(self.hwnd, delta_y);
    }

    /// Reveal a DOM keyboard-focus rectangle in the Windows document viewport.
    /// Nested CSS scroll containers have already been adjusted by op_engine.
    pub fn reveal_document_bounds(&self, top: i32, bottom: i32) {
        let mut client: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(self.hwnd, &mut client) };
        let viewport = (client.bottom - TOOLBAR_HEIGHT).max(0);
        if viewport <= 0 {
            return;
        }
        let current = SCROLL_Y.load(Ordering::SeqCst);
        let margin = 8.min(viewport / 4);
        let delta = if top < current.saturating_add(margin) {
            top.saturating_sub(current).saturating_sub(margin)
        } else if bottom > current.saturating_add(viewport).saturating_sub(margin) {
            bottom
                .saturating_sub(current.saturating_add(viewport))
                .saturating_add(margin)
        } else {
            0
        };
        if delta != 0 {
            scroll_document_delta(self.hwnd, delta);
        }
    }

    pub fn present(&self, address: &str, display_list: DisplayList) {
        // A new document invalidates the previous scrollbar thumb and capture.
        if THUMB_DRAG_ACTIVE.swap(false, Ordering::SeqCst) {
            unsafe { ReleaseCapture() };
        }
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

    /// Keep address edits and scroll position while replacing geometry/hit regions.
    pub fn present_reflow(&self, display_list: DisplayList) {
        set_display_list(display_list);
        clamp_scroll(self.hwnd);
        PAINTED_ONCE.store(false, Ordering::SeqCst);
        unsafe {
            InvalidateRect(self.hwnd, null(), 1);
            UpdateWindow(self.hwnd);
        }
    }

    /// Resize the real native client area; used by offline reflow smoke tests.
    pub fn resize_viewport(&self, width: i32, height: i32) {
        let mut outer: RECT = unsafe { zeroed() };
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetWindowRect(self.hwnd, &mut outer);
            GetClientRect(self.hwnd, &mut client);
            SetWindowPos(
                self.hwnd,
                null_mut(),
                0,
                0,
                width.max(240) + outer.right - outer.left - client.right,
                height.max(100) + TOOLBAR_HEIGHT + outer.bottom - outer.top - client.bottom,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn link_at_client_point(&self, x: i32, y: i32) -> Option<String> {
        link_at_client_point(self.hwnd, x, y)
    }

    /// Queue a native click, also used by offline end-to-end link smoke tests.
    pub fn click_first_link(&self) -> bool {
        let point = LINK_REGIONS
            .get()
            .and_then(|regions| regions.read().ok())
            .and_then(|regions| {
                regions.first().map(|region| {
                    (
                        region.bounds.left + 1,
                        region.bounds.top + 1 + TOOLBAR_HEIGHT - SCROLL_Y.load(Ordering::SeqCst),
                    )
                })
            });
        if let Some((x, y)) = point {
            if self.link_at_client_point(x, y).is_none() {
                return false;
            }
            unsafe {
                PostMessageW(
                    self.hwnd,
                    WM_LBUTTONUP,
                    0,
                    ((y as u16 as u32) << 16 | x as u16 as u32) as LPARAM,
                );
            }
            true
        } else {
            false
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
            WM_LBUTTONDOWN if message.hwnd == self.hwnd => {
                let x = (message.lParam as u16 as i16) as i32;
                let y = ((message.lParam >> 16) as u16 as i16) as i32;
                let mut client: RECT = unsafe { zeroed() };
                unsafe { GetClientRect(self.hwnd, &mut client) };
                let document_y = y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst);
                if x >= 0
                    && x < client.right
                    && y >= TOOLBAR_HEIGHT
                    && y < client.bottom
                    && scrollbar_thumb_at_document_point(x, document_y)
                {
                    THUMB_DRAG_LAST_Y.store(document_y, Ordering::SeqCst);
                    THUMB_DRAG_ACTIVE.store(true, Ordering::SeqCst);
                    unsafe { SetCapture(self.hwnd) };
                    Some(NavigationEvent::ScrollbarDragStart { x, y: document_y })
                } else if x >= 0 && x < client.right && y >= TOOLBAR_HEIGHT && y < client.bottom {
                    unsafe {
                        SetFocus(self.hwnd);
                        SetCapture(self.hwnd)
                    };
                    POINTER_PRESS_CAPTURE.store(true, Ordering::SeqCst);
                    Some(NavigationEvent::PointerDown { x, y: document_y })
                } else {
                    None
                }
            }
            WM_MOUSEMOVE
                if message.hwnd == self.hwnd && THUMB_DRAG_ACTIVE.load(Ordering::SeqCst) =>
            {
                if unsafe { GetCapture() } != self.hwnd {
                    THUMB_DRAG_ACTIVE.store(false, Ordering::SeqCst);
                    return Some(NavigationEvent::ScrollbarDragEnd);
                }
                let y = ((message.lParam >> 16) as u16 as i16) as i32;
                let document_y = y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst);
                (THUMB_DRAG_LAST_Y.swap(document_y, Ordering::SeqCst) != document_y)
                    .then_some(NavigationEvent::ScrollbarDragMove { y: document_y })
            }
            WM_MOUSEMOVE if message.hwnd == self.hwnd => {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: self.hwnd,
                    dwHoverTime: 0,
                };
                unsafe { TrackMouseEvent(&mut tracking) };
                let x = (message.lParam as u16 as i16) as i32;
                let y = ((message.lParam >> 16) as u16 as i16) as i32;
                let mut client: RECT = unsafe { zeroed() };
                unsafe { GetClientRect(self.hwnd, &mut client) };
                if x < 0 || x >= client.right || y < TOOLBAR_HEIGHT || y >= client.bottom {
                    return Some(NavigationEvent::Hover { x: -1, y: -1 });
                }
                Some(NavigationEvent::Hover {
                    x,
                    y: y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst),
                })
            }
            WM_MOUSELEAVE if message.hwnd == self.hwnd => {
                Some(NavigationEvent::Hover { x: -1, y: -1 })
            }
            WM_LBUTTONUP if message.hwnd == self.hwnd => {
                if THUMB_DRAG_ACTIVE.swap(false, Ordering::SeqCst) {
                    unsafe { ReleaseCapture() };
                    return Some(NavigationEvent::ScrollbarDragEnd);
                }
                let was_pressed = POINTER_PRESS_CAPTURE.swap(false, Ordering::SeqCst);
                if was_pressed && unsafe { GetCapture() } == self.hwnd {
                    unsafe { ReleaseCapture() };
                }
                let x = (message.lParam as u16 as i16) as i32;
                let y = ((message.lParam >> 16) as u16 as i16) as i32;
                if let Some(href) = self.link_at_client_point(x, y) {
                    Some(NavigationEvent::FollowLinkAt {
                        href,
                        x,
                        y: y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst),
                    })
                } else {
                    let mut client: RECT = unsafe { zeroed() };
                    unsafe {
                        GetClientRect(self.hwnd, &mut client);
                    }
                    (x >= 0 && x < client.right && y >= TOOLBAR_HEIGHT && y < client.bottom)
                        .then_some(NavigationEvent::Click {
                            x,
                            y: y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst),
                        })
                        .or_else(|| was_pressed.then_some(NavigationEvent::PointerCancel))
                }
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL if message.hwnd == self.hwnd => {
                let mut point = POINT {
                    x: (message.lParam as u16 as i16) as i32,
                    y: ((message.lParam >> 16) as u16 as i16) as i32,
                };
                unsafe {
                    ScreenToClient(self.hwnd, &mut point);
                }
                let mut client: RECT = unsafe { zeroed() };
                unsafe {
                    GetClientRect(self.hwnd, &mut client);
                }
                let delta = ((message.wParam >> 16) as u16 as i16) as i32;
                let delta_y = -(delta / 120) * 72;
                let horizontal = message.message == WM_MOUSEHWHEEL
                    || unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
                let distance = if message.message == WM_MOUSEHWHEEL {
                    -delta_y
                } else {
                    delta_y
                };
                (delta_y != 0
                    && point.x >= 0
                    && point.x < client.right
                    && point.y >= TOOLBAR_HEIGHT
                    && point.y < client.bottom)
                    .then_some(if horizontal {
                        NavigationEvent::HorizontalScroll {
                            x: point.x,
                            y: point.y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst),
                            delta_x: distance,
                        }
                    } else {
                        NavigationEvent::Scroll {
                            x: point.x,
                            y: point.y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst),
                            delta_y,
                        }
                    })
            }
            WM_KEYDOWN
                if message.wParam == VK_RETURN as usize
                    && message.hwnd == unsafe { GetDlgItem(self.hwnd, ADDRESS) } =>
            {
                Some(NavigationEvent::Navigate(self.address()))
            }
            WM_CHAR if message.hwnd == self.hwnd => {
                let unit = u16::try_from(message.wParam).ok()?;
                if (0xD800..=0xDBFF).contains(&unit) {
                    PENDING_UTF16_HIGH.store(usize::from(unit), Ordering::SeqCst);
                    None
                } else {
                    let pending = PENDING_UTF16_HIGH.swap(0, Ordering::SeqCst);
                    let code_point = if (0xDC00..=0xDFFF).contains(&unit)
                        && (0xD800..=0xDBFF).contains(&(pending as u16))
                    {
                        0x10000 + (((pending as u32 - 0xD800) << 10) | (u32::from(unit) - 0xDC00))
                    } else {
                        u32::from(unit)
                    };
                    char::from_u32(code_point)
                        .filter(|ch| !ch.is_control())
                        .map(|ch| NavigationEvent::TextInput(ch.to_string()))
                }
            }
            WM_KEYDOWN if message.wParam == VK_F5 as usize => Some(NavigationEvent::Reload),
            WM_KEYDOWN | WM_KEYUP
                if message.hwnd == self.hwnd
                    && !(message.wParam == VK_L as usize
                        && unsafe { GetKeyState(VK_CONTROL as i32) } < 0) =>
            {
                let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
                let ctrl = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
                native_key_name(message.wParam, shift).map(|(key, code)| {
                    if message.message == WM_KEYDOWN {
                        NavigationEvent::KeyDown {
                            key,
                            code,
                            shift,
                            ctrl,
                            repeat: (message.lParam as u32 & (1 << 30)) != 0,
                        }
                    } else {
                        NavigationEvent::KeyUp {
                            key,
                            code,
                            shift,
                            ctrl,
                        }
                    }
                })
            }
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
            WM_TIMER if message.hwnd == self.hwnd && message.wParam == RESIZE_TIMER => {
                unsafe {
                    KillTimer(self.hwnd, RESIZE_TIMER);
                }
                Some(NavigationEvent::Resize)
            }
            CANCEL_THUMB_DRAG if message.hwnd == self.hwnd => {
                Some(NavigationEvent::ScrollbarDragEnd)
            }
            CANCEL_POINTER_PRESS if message.hwnd == self.hwnd => {
                Some(NavigationEvent::PointerCancel)
            }
            _ => None,
        }
    }
}

/// Keyboard semantics for original page input (address edit remains native).
fn native_key_name(vk: usize, shift: bool) -> Option<(String, String)> {
    let named = match vk as u32 {
        9 => Some(("Tab", "Tab")),
        13 => Some(("Enter", "Enter")),
        27 => Some(("Escape", "Escape")),
        8 => Some(("Backspace", "Backspace")),
        32 => Some((" ", "Space")),
        37 => Some(("ArrowLeft", "ArrowLeft")),
        38 => Some(("ArrowUp", "ArrowUp")),
        39 => Some(("ArrowRight", "ArrowRight")),
        40 => Some(("ArrowDown", "ArrowDown")),
        46 => Some(("Delete", "Delete")),
        _ => None,
    };
    if let Some((key, code)) = named {
        return Some((key.to_owned(), code.to_owned()));
    }
    let ch = u8::try_from(vk).ok()?;
    if ch.is_ascii_uppercase() {
        return Some((
            ((if shift { ch } else { ch.to_ascii_lowercase() }) as char).to_string(),
            format!("Key{}", ch as char),
        ));
    }
    if ch.is_ascii_digit() {
        return Some(((ch as char).to_string(), format!("Digit{}", ch as char)));
    }
    None
}

fn scroll_document_delta(hwnd: HWND, delta_y: i32) {
    let limit = max_scroll(hwnd);
    let old = SCROLL_Y.load(Ordering::SeqCst);
    let next = old.saturating_add(delta_y).clamp(0, limit);
    if next != old {
        SCROLL_Y.store(next, Ordering::SeqCst);
        unsafe {
            InvalidateRect(hwnd, null(), 1);
        }
    }
}

fn max_scroll(hwnd: HWND) -> i32 {
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
    }
    let height = DISPLAY_LIST
        .get()
        .and_then(|storage| storage.read().ok())
        .and_then(|list| {
            list.commands
                .iter()
                .filter_map(|command| match command {
                    PaintCommand::FillRect { y, height, .. } => Some(y + height),
                    _ => None,
                })
                .max()
        })
        .unwrap_or(0);
    (height - (rect.bottom - TOOLBAR_HEIGHT).max(0)).max(0)
}

fn clamp_scroll(hwnd: HWND) {
    let old = SCROLL_Y.load(Ordering::SeqCst);
    SCROLL_Y.store(old.clamp(0, max_scroll(hwnd)), Ordering::SeqCst);
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
    if let Ok(mut regions) = LINK_REGIONS.get_or_init(|| RwLock::new(Vec::new())).write() {
        regions.clear();
    }
    let storage = DISPLAY_LIST.get_or_init(|| RwLock::new(DisplayList::default()));

    if let Ok(mut current) = storage.write() {
        *current = display_list;
    }
}

/// Scrollbar overlays take priority over links painted beneath their tracks.
/// The UI uses the painted thumb rectangles to start native mouse capture.
/// The original page engine independently resolves the same layout geometry.
fn scrollbar_hit(command: &PaintCommand, x: i32, y: i32, thumb_only: bool) -> bool {
    match command {
        PaintCommand::Clipped { rect, command } => {
            x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height)
                && scrollbar_hit(command, x, y, thumb_only)
        }
        PaintCommand::Scrollbar { track, thumb } => {
            let rect = if thumb_only { thumb } else { track };
            x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height)
        }
        PaintCommand::HorizontalScrollbar { track, .. } if !thumb_only => {
            x >= track.x
                && x < track.x.saturating_add(track.width)
                && y >= track.y
                && y < track.y.saturating_add(track.height)
        }
        _ => false,
    }
}

fn scrollbar_thumb_at_document_point(x: i32, y: i32) -> bool {
    DISPLAY_LIST
        .get()
        .and_then(|storage| storage.read().ok())
        .is_some_and(|list| {
            list.commands
                .iter()
                .rev()
                .any(|command| scrollbar_hit(command, x, y, true))
        })
}

fn scrollbar_at_document_point(x: i32, y: i32) -> bool {
    DISPLAY_LIST
        .get()
        .and_then(|storage| storage.read().ok())
        .is_some_and(|list| {
            list.commands
                .iter()
                .rev()
                .any(|command| scrollbar_hit(command, x, y, false))
        })
}

fn link_at_client_point(hwnd: HWND, x: i32, y: i32) -> Option<String> {
    let mut client: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    if x < 0 || x >= client.right || y < TOOLBAR_HEIGHT || y >= client.bottom {
        return None;
    }
    let document_y = y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst);
    if scrollbar_at_document_point(x, document_y) {
        return None;
    }
    LINK_REGIONS
        .get()?
        .read()
        .ok()?
        .iter()
        .rev() // Prefer the later-painted link when link rectangles overlap.
        .find(|region| {
            x >= region.bounds.left
                && x < region.bounds.right
                && document_y >= region.bounds.top
                && document_y < region.bounds.bottom
        })
        .map(|region| region.href.clone())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CAPTURECHANGED => {
            if THUMB_DRAG_ACTIVE.swap(false, Ordering::SeqCst) {
                unsafe { PostMessageW(hwnd, CANCEL_THUMB_DRAG, 0, 0) };
            }
            if POINTER_PRESS_CAPTURE.swap(false, Ordering::SeqCst) {
                unsafe { PostMessageW(hwnd, CANCEL_POINTER_PRESS, 0, 0) };
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_CANCELMODE => {
            if THUMB_DRAG_ACTIVE.load(Ordering::SeqCst)
                || POINTER_PRESS_CAPTURE.load(Ordering::SeqCst)
            {
                unsafe { ReleaseCapture() };
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_SETCURSOR if lparam as u16 as i16 as i32 == HTCLIENT as i32 => {
            let mut point: POINT = unsafe { zeroed() };
            unsafe {
                GetCursorPos(&mut point);
                ScreenToClient(hwnd, &mut point);
            }
            if link_at_client_point(hwnd, point.x, point.y).is_some() {
                unsafe {
                    SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                }
                return 1;
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
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
            unsafe {
                KillTimer(hwnd, RESIZE_TIMER);
                if wparam != SIZE_MINIMIZED as usize {
                    clamp_scroll(hwnd);
                    SetTimer(hwnd, RESIZE_TIMER, 120, None);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as u16 as i16) as i32;
            scroll_document_delta(hwnd, -(delta / 120) * 72);
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

    let mut link_regions = Vec::new();
    PAINTED_IMAGES.store(0, Ordering::SeqCst);
    if let Some(storage) = DISPLAY_LIST.get()
        && let Ok(display_list) = storage.read()
    {
        paint_commands(hdc, &display_list.commands, &mut link_regions);
    }
    if let Ok(mut regions) = LINK_REGIONS.get_or_init(|| RwLock::new(Vec::new())).write() {
        *regions = link_regions;
    }

    unsafe {
        EndPaint(hwnd, &paint);
    }

    PAINTED_ONCE.store(true, Ordering::SeqCst);
}

fn paint_commands(hdc: *mut c_void, commands: &[PaintCommand], links: &mut Vec<LinkRegion>) {
    paint_commands_inner(hdc, commands, links, 0);
}

fn paint_commands_inner(
    hdc: *mut c_void,
    commands: &[PaintCommand],
    links: &mut Vec<LinkRegion>,
    depth: usize,
) {
    let mut index = 0usize;
    while index < commands.len() {
        if let PaintCommand::BeginLayer { opacity, invert } = commands[index] {
            let mut nesting = 1usize;
            let mut end = index + 1;
            while end < commands.len() && nesting > 0 {
                match commands[end] {
                    PaintCommand::BeginLayer { .. } => nesting += 1,
                    PaintCommand::EndLayer => nesting -= 1,
                    _ => {}
                }
                end += 1;
            }
            if nesting == 0 && depth < 32 {
                composite_layer(
                    hdc,
                    &commands[index + 1..end - 1],
                    opacity,
                    invert,
                    links,
                    depth + 1,
                );
            } else if nesting == 0 && opacity > 0 {
                paint_unmodified_group(hdc, &commands[index + 1..end - 1], links);
            }
            index = end;
        } else {
            if !matches!(commands[index], PaintCommand::EndLayer) {
                paint_command(hdc, &commands[index], links);
            }
            index += 1;
        }
    }
}

// Preserve visible document contents if bounded offscreen compositing cannot
// allocate a surface. Effects may be omitted, but content is never dropped.
fn paint_unmodified_group(
    target: *mut c_void,
    commands: &[PaintCommand],
    links: &mut Vec<LinkRegion>,
) {
    for command in commands {
        if !matches!(
            command,
            PaintCommand::BeginLayer { .. } | PaintCommand::EndLayer
        ) {
            paint_command(target, command, links);
        }
    }
}

fn layer_bounds(commands: &[PaintCommand]) -> Option<(i32, i32, i32, i32)> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for command in commands {
        let rect = match command {
            PaintCommand::FillRect {
                x,
                y,
                width,
                height,
                ..
            }
            | PaintCommand::BackgroundImage {
                x,
                y,
                width,
                height,
                ..
            }
            | PaintCommand::Image {
                x,
                y,
                width,
                height,
                ..
            } => (*x, *y, *width, *height),
            PaintCommand::Text {
                x,
                y,
                font_size,
                text,
                ..
            } => (
                *x,
                y.saturating_sub(*font_size),
                (text.chars().count() as i32)
                    .saturating_mul(*font_size)
                    .saturating_mul(2)
                    .max(1),
                font_size.saturating_mul(3).max(1),
            ),
            PaintCommand::Clipped { rect, command } => {
                let Some((x, y, right, bottom)) =
                    layer_bounds(std::slice::from_ref(command.as_ref()))
                else {
                    continue;
                };
                let clipped = op_paint::ClipRect {
                    x,
                    y,
                    width: right.saturating_sub(x),
                    height: bottom.saturating_sub(y),
                }
                .intersect(*rect);
                (clipped.x, clipped.y, clipped.width, clipped.height)
            }
            _ => continue,
        };
        if rect.2 <= 0 || rect.3 <= 0 {
            continue;
        }
        let right = rect.0.saturating_add(rect.2);
        let bottom = rect.1.saturating_add(rect.3);
        bounds = Some(match bounds {
            Some((left, top, old_right, old_bottom)) => (
                left.min(rect.0),
                top.min(rect.1),
                old_right.max(right),
                old_bottom.max(bottom),
            ),
            None => (rect.0, rect.1, right, bottom),
        });
    }
    bounds
}

fn composite_layer(
    target: *mut c_void,
    commands: &[PaintCommand],
    opacity: u8,
    invert: u8,
    links: &mut Vec<LinkRegion>,
    depth: usize,
) {
    if opacity == 0 {
        return;
    }
    let Some((x, y, right, bottom)) = layer_bounds(commands) else {
        return;
    };
    let width = right.saturating_sub(x);
    let height = bottom.saturating_sub(y);
    if width <= 0
        || height <= 0
        || width > 4096
        || height > 4096
        || (width as u64) * (height as u64) > 4_194_304
    {
        paint_unmodified_group(target, commands, links);
        return;
    }

    let Some(white) = raster::Surface::new(target, width, height) else {
        paint_unmodified_group(target, commands, links);
        return;
    };
    let Some(black) = raster::Surface::new(target, width, height) else {
        paint_unmodified_group(target, commands, links);
        return;
    };
    white.clear_white();
    black.clear_black();
    unsafe {
        SetViewportOrgEx(white.dc, x.saturating_neg(), y.saturating_neg(), null_mut());
        SetViewportOrgEx(black.dc, x.saturating_neg(), y.saturating_neg(), null_mut());
    }
    let mut group_links = Vec::new();
    paint_commands_inner(black.dc, commands, &mut group_links, depth);
    paint_commands_inner(white.dc, commands, &mut Vec::new(), depth);
    unsafe {
        windows_sys::Win32::Graphics::Gdi::GdiFlush();
    }
    let black_pixels = black.pixels();
    let white_pixels = white.pixels();
    let mut result = vec![0u8; black_pixels.len()];
    for (index, (black_pixel, white_pixel)) in black_pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(white_pixels.as_chunks::<4>().0.iter())
        .enumerate()
    {
        let lost_white = (0..3)
            .map(|channel| white_pixel[channel] as i32 - black_pixel[channel] as i32)
            .max()
            .unwrap_or(255)
            .clamp(0, 255);
        let alpha = 255 - lost_white;
        let scaled_alpha = (alpha * opacity as i32 + 127) / 255;
        for channel in 0..3 {
            let premult = (black_pixel[channel] as i32).clamp(0, alpha);
            let filtered =
                (premult * (255 - invert as i32) + (alpha - premult) * invert as i32 + 127) / 255;
            result[4 * index + channel] =
                ((filtered * opacity as i32 + 127) / 255).clamp(0, scaled_alpha) as u8;
        }
        result[4 * index + 3] = scaled_alpha as u8;
    }
    if let Ok(raster_image) =
        op_paint::RasterImage::from_premultiplied_bgra(width as u32, height as u32, result)
    {
        let composite = PaintCommand::Image {
            x,
            y,
            width,
            height,
            image: std::sync::Arc::new(raster_image),
            href: None,
        };
        paint_command(target, &composite, &mut Vec::new());
        links.extend(group_links);
    }
}

fn paint_command(hdc: *mut c_void, command: &PaintCommand, link_regions: &mut Vec<LinkRegion>) {
    match command {
        PaintCommand::Scrollbar { track, thumb }
        | PaintCommand::HorizontalScrollbar { track, thumb } => {
            for (rect, color) in [
                (
                    *track,
                    Color {
                        r: 234,
                        g: 237,
                        b: 241,
                    },
                ),
                (
                    *thumb,
                    Color {
                        r: 120,
                        g: 130,
                        b: 144,
                    },
                ),
            ] {
                paint_command(
                    hdc,
                    &PaintCommand::FillRect {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                        color,
                    },
                    link_regions,
                );
            }
        }
        PaintCommand::Clipped { rect, command } => {
            if rect.width <= 0 || rect.height <= 0 {
                return;
            }
            let saved = unsafe { SaveDC(hdc) };
            if saved == 0 {
                return;
            }
            unsafe {
                IntersectClipRect(
                    hdc,
                    rect.x,
                    rect.y,
                    rect.x.saturating_add(rect.width),
                    rect.y.saturating_add(rect.height),
                );
            }
            let link_start = link_regions.len();
            paint_command(hdc, command, link_regions);
            for region in &mut link_regions[link_start..] {
                region.bounds.left = region.bounds.left.max(rect.x);
                region.bounds.top = region.bounds.top.max(rect.y);
                region.bounds.right = region.bounds.right.min(rect.x.saturating_add(rect.width));
                region.bounds.bottom = region.bounds.bottom.min(rect.y.saturating_add(rect.height));
            }
            link_regions.retain(|region| {
                region.bounds.right > region.bounds.left && region.bounds.bottom > region.bounds.top
            });
            unsafe {
                RestoreDC(hdc, saved);
            }
        }
        PaintCommand::BeginLayer { .. } | PaintCommand::EndLayer => {}
        PaintCommand::BackgroundImage { .. } => {
            raster::paint_background(hdc, command);
        }
        PaintCommand::Image {
            x,
            y,
            width,
            height,
            href,
            ..
        } => {
            if raster::paint(hdc, command) {
                PAINTED_IMAGES.fetch_add(1, Ordering::SeqCst);
                if let Some(href) = href {
                    link_regions.push(LinkRegion {
                        bounds: RECT {
                            left: *x,
                            top: *y,
                            right: *x + *width,
                            bottom: *y + *height,
                        },
                        href: href.clone(),
                    });
                }
            }
        }
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
            italic,
            underline,
            line_through,
            letter_spacing,
            word_spacing,
            color,
            links,
        } => {
            let _guard = op_paint::GDI_TEXT_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let face = wide(op_paint::TEXT_FONT_FAMILY);
            let weight = if *bold { FW_BOLD } else { FW_NORMAL } as i32;

            let font = unsafe {
                CreateFontW(
                    -*font_size,
                    0,
                    0,
                    0,
                    weight,
                    u32::from(*italic),
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

            let mut cursor_x = *x;
            let mut offset = 0;
            for link in links {
                if link.start < offset
                    || link.start >= link.end
                    || !text.is_char_boundary(link.start)
                    || !text.is_char_boundary(link.end)
                {
                    continue;
                }
                let Some(plain) = text.get(offset..link.start) else {
                    continue;
                };
                let Some(label) = text.get(link.start..link.end) else {
                    continue;
                };
                let plain_width = paint_text_segment(
                    hdc,
                    cursor_x,
                    *y,
                    plain,
                    *color,
                    *letter_spacing,
                    *word_spacing,
                );
                paint_text_decorations(
                    hdc,
                    cursor_x,
                    *y,
                    plain_width,
                    *font_size,
                    *color,
                    (*underline, *line_through),
                );
                cursor_x += plain_width;
                let width = paint_text_segment(
                    hdc,
                    cursor_x,
                    *y,
                    label,
                    *color,
                    *letter_spacing,
                    *word_spacing,
                );
                paint_text_decorations(
                    hdc,
                    cursor_x,
                    *y,
                    width,
                    *font_size,
                    *color,
                    (*underline, *line_through),
                );
                link_regions.push(LinkRegion {
                    bounds: RECT {
                        left: cursor_x,
                        top: *y,
                        right: cursor_x + width,
                        bottom: *y + ((*font_size as f32) * 1.35).round() as i32,
                    },
                    href: link.href.clone(),
                });
                cursor_x += width;
                offset = link.end;
            }
            let tail_width = paint_text_segment(
                hdc,
                cursor_x,
                *y,
                &text[offset..],
                *color,
                *letter_spacing,
                *word_spacing,
            );
            paint_text_decorations(
                hdc,
                cursor_x,
                *y,
                tail_width,
                *font_size,
                *color,
                (*underline, *line_through),
            );

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

fn paint_text_decorations(
    hdc: *mut c_void,
    x: i32,
    y: i32,
    width: i32,
    font_size: i32,
    color: Color,
    decoration: (bool, bool),
) {
    let (underline, line_through) = decoration;
    if width <= 0 || (!underline && !line_through) {
        return;
    }
    let brush = unsafe { CreateSolidBrush(color_ref(color)) };
    if brush.is_null() {
        return;
    }
    if underline {
        let rect = RECT {
            left: x,
            top: y + font_size + 2,
            right: x + width,
            bottom: y + font_size + 3,
        };
        unsafe { FillRect(hdc, &rect, brush) };
    }
    if line_through {
        let middle = y + (font_size * 11 / 20);
        let rect = RECT {
            left: x,
            top: middle,
            right: x + width,
            bottom: middle + 1,
        };
        unsafe { FillRect(hdc, &rect, brush) };
    }
    unsafe { DeleteObject(brush) };
}

fn paint_text_segment(
    hdc: *mut c_void,
    x: i32,
    y: i32,
    text: &str,
    color: Color,
    letter_spacing: i32,
    word_spacing: i32,
) -> i32 {
    let wide_text: Vec<u16> = text.encode_utf16().collect();
    let mut size: SIZE = unsafe { zeroed() };
    if wide_text.is_empty() {
        return 0;
    }
    unsafe {
        SetTextColor(hdc, color_ref(color));
        GetTextExtentPoint32W(hdc, wide_text.as_ptr(), wide_text.len() as i32, &mut size);
    }

    let char_count = text.chars().count();
    let spaces = text.chars().filter(|ch| *ch == ' ').count() as i32;
    let target_width = size
        .cx
        .saturating_add(letter_spacing.saturating_mul(char_count.saturating_sub(1) as i32))
        .saturating_add(word_spacing.saturating_mul(spaces))
        .max(0);

    if letter_spacing == 0 && word_spacing == 0 {
        unsafe { TextOutW(hdc, x, y, wide_text.as_ptr(), wide_text.len() as i32) };
        return target_width;
    }

    let mut cursor = x;
    for (index, ch) in text.chars().enumerate() {
        let units: Vec<u16> = ch.encode_utf16(&mut [0; 2]).to_vec();
        let mut char_size: SIZE = unsafe { zeroed() };
        unsafe {
            TextOutW(hdc, cursor, y, units.as_ptr(), units.len() as i32);
            GetTextExtentPoint32W(hdc, units.as_ptr(), units.len() as i32, &mut char_size);
        }
        cursor = cursor.saturating_add(char_size.cx);
        if ch == ' ' {
            cursor = cursor.saturating_add(word_spacing);
        }
        if index + 1 < char_count {
            cursor = cursor.saturating_add(letter_spacing);
        }
    }
    target_width
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

    // Native windows share class registration, capture and display-list state.
    static WINDOW_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn r12e_horizontal_scrollbar_paints_real_gdi_track_and_thumb() {
        let bar = PaintCommand::HorizontalScrollbar {
            track: op_paint::ClipRect {
                x: 12,
                y: 48,
                width: 90,
                height: 10,
            },
            thumb: op_paint::ClipRect {
                x: 35,
                y: 49,
                width: 26,
                height: 8,
            },
        };
        assert!(scrollbar_hit(&bar, 40, 52, false));
        assert!(!scrollbar_hit(&bar, 40, 52, true));
        let width = 120usize;
        let pixels = render_display_list_to_bgra(
            &DisplayList {
                commands: vec![
                    PaintCommand::FillRect {
                        x: 0,
                        y: 0,
                        width: 120,
                        height: 70,
                        color: Color::WHITE,
                    },
                    bar,
                ],
            },
            120,
            70,
        )
        .unwrap();
        let sample = |x: usize, y: usize| {
            let offset = (y * width + x) * 4;
            &pixels[offset..offset + 3]
        };
        assert_eq!(sample(45, 53), &[144, 130, 120]);
        assert_eq!(sample(84, 53), &[241, 237, 234]);
        assert_eq!(sample(84, 63), &[255, 255, 255]);
    }

    #[test]
    fn r12d_clipped_scrollbar_paints_only_visible_pixels_and_rejects_hidden_hits() {
        let inner = PaintCommand::Scrollbar {
            track: op_paint::ClipRect {
                x: 20,
                y: 10,
                width: 10,
                height: 60,
            },
            thumb: op_paint::ClipRect {
                x: 21,
                y: 11,
                width: 8,
                height: 18,
            },
        };
        let clipped = PaintCommand::Clipped {
            rect: op_paint::ClipRect {
                x: 0,
                y: 0,
                width: 64,
                height: 23,
            },
            command: Box::new(inner),
        };
        assert!(scrollbar_hit(&clipped, 24, 15, true));
        assert!(!scrollbar_hit(&clipped, 24, 28, true));
        assert!(!scrollbar_hit(&clipped, 24, 60, false));
        assert!(scrollbar_hit(&clipped, 24, 20, false));
        let width = 64usize;
        let pixels = render_display_list_to_bgra(
            &DisplayList {
                commands: vec![
                    PaintCommand::FillRect {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 80,
                        color: Color::WHITE,
                    },
                    clipped,
                ],
            },
            64,
            80,
        )
        .unwrap();
        let rgb = |x: usize, y: usize| -> &[u8] {
            let offset = (y * width + x) * 4;
            &pixels[offset..offset + 3]
        };
        assert_eq!(rgb(24, 15), &[144, 130, 120]);
        assert_eq!(rgb(24, 35), &[255, 255, 255]);
    }

    #[test]
    fn r12b_scrollbar_track_and_thumb_render_to_distinct_native_pixels() {
        let width = 64usize;
        let pixels = render_display_list_to_bgra(
            &DisplayList {
                commands: vec![
                    PaintCommand::FillRect {
                        x: 0,
                        y: 0,
                        width: width as i32,
                        height: 90,
                        color: Color::WHITE,
                    },
                    PaintCommand::Scrollbar {
                        track: op_paint::ClipRect {
                            x: 20,
                            y: 10,
                            width: 10,
                            height: 60,
                        },
                        thumb: op_paint::ClipRect {
                            x: 21,
                            y: 11,
                            width: 8,
                            height: 20,
                        },
                    },
                ],
            },
            width as i32,
            90,
        )
        .unwrap();
        let sample = |x: usize, y: usize| {
            let offset = (y * width + x) * 4;
            &pixels[offset..offset + 3]
        };
        assert_eq!(sample(24, 18), &[144, 130, 120]);
        assert_eq!(sample(24, 55), &[241, 237, 234]);
        assert_eq!(sample(16, 55), &[255, 255, 255]);
    }

    #[test]
    fn keyboard_focus_reveals_bottom_and_then_top_of_native_document() {
        let _window_lock = WINDOW_TEST_LOCK.lock().unwrap();
        let window = NativeBrowserWindow::create("Focus scroll test", DisplayList::default())
            .expect("native window");
        window.present_reflow(DisplayList {
            commands: vec![PaintCommand::FillRect {
                x: 0,
                y: 0,
                width: 450,
                height: 1700,
                color: Color::WHITE,
            }],
        });
        assert_eq!(SCROLL_Y.load(Ordering::SeqCst), 0);
        window.reveal_document_bounds(1450, 1480);
        assert!(
            SCROLL_Y.load(Ordering::SeqCst) > 0,
            "Tab to distant content should scroll the native document"
        );
        let bottom_offset = SCROLL_Y.load(Ordering::SeqCst);
        window.reveal_document_bounds(1450, 1480);
        assert_eq!(
            SCROLL_Y.load(Ordering::SeqCst),
            bottom_offset,
            "visible target should not jitter"
        );
        window.reveal_document_bounds(12, 30);
        assert_eq!(
            SCROLL_Y.load(Ordering::SeqCst),
            4,
            "Shift+Tab near top should move viewport back"
        );
        window.close();
    }

    #[test]
    fn native_wm_char_translates_cyrillic_and_surrogate_pairs_without_touching_url_bar() {
        let _lock = WINDOW_TEST_LOCK.lock().unwrap();
        let window =
            NativeBrowserWindow::create("Unicode typing test", DisplayList::default()).unwrap();
        let mut message: MSG = unsafe { zeroed() };
        message.hwnd = window.hwnd;
        message.message = WM_CHAR;
        message.wParam = 0x043F; // п
        assert_eq!(
            window.navigation_event(&message),
            Some(NavigationEvent::TextInput("п".into()))
        );
        message.wParam = 0xD83D;
        assert_eq!(window.navigation_event(&message), None);
        message.wParam = 0xDE0A;
        assert_eq!(
            window.navigation_event(&message),
            Some(NavigationEvent::TextInput("😊".into()))
        );
        message.hwnd = unsafe { GetDlgItem(window.hwnd, ADDRESS) };
        message.wParam = 0x0440;
        assert_eq!(window.navigation_event(&message), None);
        window.close();
    }

    #[test]
    fn native_page_keyboard_shortcuts_preserve_toolbar_address_input() {
        let _window_lock = WINDOW_TEST_LOCK.lock().unwrap();
        let window = NativeBrowserWindow::create("Keyboard focus test", DisplayList::default())
            .expect("native window");
        let mut key: MSG = unsafe { zeroed() };
        key.hwnd = window.hwnd;
        key.message = WM_KEYDOWN;
        key.wParam = 9;
        assert!(matches!(
            window.navigation_event(&key),
            Some(NavigationEvent::KeyDown { ref key, .. }) if key == "Tab"
        ));
        key.wParam = VK_RETURN as usize;
        assert!(matches!(
            window.navigation_event(&key),
            Some(NavigationEvent::KeyDown { ref key, .. }) if key == "Enter"
        ));
        key.message = WM_KEYUP;
        assert!(matches!(
            window.navigation_event(&key),
            Some(NavigationEvent::KeyUp { ref key, .. }) if key == "Enter"
        ));
        key.message = WM_KEYDOWN;
        key.wParam = VK_F5 as usize;
        assert_eq!(window.navigation_event(&key), Some(NavigationEvent::Reload));
        key.hwnd = unsafe { GetDlgItem(window.hwnd, ADDRESS) };
        key.wParam = 9;
        assert_eq!(
            window.navigation_event(&key),
            None,
            "Tab in address field is not a page shortcut"
        );
        key.wParam = VK_RETURN as usize;
        assert!(matches!(
            window.navigation_event(&key),
            Some(NavigationEvent::Navigate(_))
        ));
        window.close();
    }

    #[test]
    fn native_pointer_capture_reports_press_and_cancel_after_off_page_release() {
        let _window_lock = WINDOW_TEST_LOCK.lock().unwrap();
        let window = NativeBrowserWindow::create("Press capture test", DisplayList::default())
            .expect("native window");
        let message_at = |message, x: i16, y: i16| -> MSG {
            let mut msg: MSG = unsafe { zeroed() };
            msg.hwnd = window.hwnd;
            msg.message = message;
            msg.lParam = ((u32::from(y as u16) << 16) | u32::from(x as u16)) as LPARAM;
            msg
        };
        let inside_y = (TOOLBAR_HEIGHT + 16) as i16;
        assert_eq!(
            window.navigation_event(&message_at(WM_LBUTTONDOWN, 70, inside_y)),
            Some(NavigationEvent::PointerDown { x: 70, y: 16 })
        );
        assert!(POINTER_PRESS_CAPTURE.load(Ordering::SeqCst));
        assert_eq!(unsafe { GetCapture() }, window.hwnd);
        assert_eq!(
            window.navigation_event(&message_at(WM_LBUTTONUP, 70, inside_y)),
            Some(NavigationEvent::Click { x: 70, y: 16 })
        );
        assert!(!POINTER_PRESS_CAPTURE.load(Ordering::SeqCst));
        assert_ne!(unsafe { GetCapture() }, window.hwnd);

        assert_eq!(
            window.navigation_event(&message_at(WM_LBUTTONDOWN, 70, inside_y)),
            Some(NavigationEvent::PointerDown { x: 70, y: 16 })
        );
        assert_eq!(
            window.navigation_event(&message_at(WM_LBUTTONUP, -4, 5)),
            Some(NavigationEvent::PointerCancel)
        );
        assert!(!POINTER_PRESS_CAPTURE.load(Ordering::SeqCst));
        window.close();
    }

    #[test]
    fn native_controls_dispatch_navigation_and_repaint() {
        let _window_lock = WINDOW_TEST_LOCK.lock().unwrap();
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
            if matches!(event, NavigationEvent::Hover { .. }) {
                return;
            }
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
                    window.present(
                        "http://example.test/nested/index",
                        DisplayList {
                            commands: vec![
                                PaintCommand::FillRect {
                                    x: 0,
                                    y: 0,
                                    width: 1200,
                                    height: 2000,
                                    color: Color::WHITE,
                                },
                                PaintCommand::Text {
                                    x: 32,
                                    y: 120,
                                    text: "Before link after".into(),
                                    font_size: 18,
                                    bold: false,
                                    italic: false,
                                    underline: false,
                                    line_through: false,
                                    letter_spacing: 0,
                                    word_spacing: 0,
                                    color: Color::BLACK,
                                    links: vec![op_paint::LinkSpan {
                                        start: 7,
                                        end: 11,
                                        href: "../target".into(),
                                    }],
                                },
                            ],
                        },
                    );
                    assert!(
                        window
                            .link_at_client_point(32, TOOLBAR_HEIGHT + 121)
                            .is_none()
                    );
                    assert!(window.link_at_client_point(33, 40).is_none());
                    let (left, right) = {
                        let regions = LINK_REGIONS.get().unwrap().read().unwrap();
                        (regions[0].bounds.left, regions[0].bounds.right)
                    };
                    assert!(
                        window
                            .link_at_client_point(left - 1, TOOLBAR_HEIGHT + 121)
                            .is_none()
                    );
                    assert!(
                        window
                            .link_at_client_point(right, TOOLBAR_HEIGHT + 121)
                            .is_none()
                    );
                    unsafe {
                        SendMessageW(
                            window.hwnd,
                            WM_MOUSEWHEEL,
                            ((-120i16 as u16) as usize) << 16,
                            0,
                        );
                        UpdateWindow(window.hwnd);
                    }
                    assert_eq!(
                        window.link_at_client_point(left + 1, TOOLBAR_HEIGHT + 121 - 72),
                        Some("../target".into())
                    );
                    assert!(window.click_first_link());
                }
                6 => {
                    assert!(matches!(event, NavigationEvent::FollowLinkAt { ref href, .. } if href == "../target"));
                    let image =
                        op_paint::RasterImage::from_premultiplied_bgra(1, 1, vec![255; 4]).unwrap();
                    window.present(
                        "http://example.test/image",
                        DisplayList {
                            commands: vec![
                                PaintCommand::FillRect {
                                    x: 0,
                                    y: 0,
                                    width: 1200,
                                    height: 2000,
                                    color: Color::WHITE,
                                },
                                PaintCommand::Image {
                                    x: 32,
                                    y: 120,
                                    width: 40,
                                    height: 20,
                                    image: std::sync::Arc::new(image),
                                    href: Some("../image".into()),
                                },
                            ],
                        },
                    );
                    assert_eq!(window.painted_image_count(), 1);
                    assert_eq!(
                        window.link_at_client_point(33, TOOLBAR_HEIGHT + 121),
                        Some("../image".into())
                    );
                    assert!(
                        window
                            .link_at_client_point(72, TOOLBAR_HEIGHT + 121)
                            .is_none()
                    );
                    unsafe {
                        SendMessageW(
                            window.hwnd,
                            WM_MOUSEWHEEL,
                            ((-120i16 as u16) as usize) << 16,
                            0,
                        );
                        UpdateWindow(window.hwnd);
                    }
                    assert_eq!(
                        window.link_at_client_point(33, TOOLBAR_HEIGHT + 121 - 72),
                        Some("../image".into())
                    );
                    assert!(window.click_first_link());
                }
                7 => {
                    assert!(matches!(event, NavigationEvent::FollowLinkAt { ref href, .. } if href == "../image"));
                    let old_scroll = SCROLL_Y.load(Ordering::SeqCst);
                    window.set_address("address edit survives reflow");
                    window.present_reflow(DisplayList {
                        commands: vec![
                            PaintCommand::FillRect {
                                x: 0,
                                y: 0,
                                width: 1200,
                                height: 2000,
                                color: Color::WHITE,
                            },
                            PaintCommand::Text {
                                x: 32,
                                y: 200,
                                text: "New link".into(),
                                font_size: 18,
                                bold: false,
                                italic: false,
                                underline: false,
                                line_through: false,
                                letter_spacing: 0,
                                word_spacing: 0,
                                color: Color::BLACK,
                                links: vec![op_paint::LinkSpan {
                                    start: 0,
                                    end: 8,
                                    href: "new".into(),
                                }],
                            },
                        ],
                    });
                    assert_eq!(SCROLL_Y.load(Ordering::SeqCst), old_scroll);
                    assert_eq!(window.address(), "address edit survives reflow");
                    assert!(
                        window
                            .link_at_client_point(33, TOOLBAR_HEIGHT + 121 - old_scroll)
                            .is_none()
                    );
                    assert_eq!(
                        window.link_at_client_point(33, TOOLBAR_HEIGHT + 201 - old_scroll),
                        Some("new".into())
                    );
                    window.present_reflow(DisplayList {
                        commands: vec![PaintCommand::FillRect {
                            x: 0,
                            y: 0,
                            width: 1200,
                            height: 120,
                            color: Color::WHITE,
                        }],
                    });
                    assert_eq!(SCROLL_Y.load(Ordering::SeqCst), 0);
                    assert_eq!(window.address(), "address edit survives reflow");
                    window.present("http://example.test/replaced", DisplayList::default());
                    assert_eq!(window.painted_image_count(), 0);
                    assert!(!window.click_first_link());
                    window.resize_viewport(640, 500);
                    window.resize_viewport(320, 400);
                }
                8 => {
                    assert_eq!(event, NavigationEvent::Resize);
                    assert_eq!(window.viewport_size(), (320, 400));
                    window.present_reflow(DisplayList {
                        commands: vec![
                            PaintCommand::FillRect {
                                x: 0,
                                y: 0,
                                width: 320,
                                height: 250,
                                color: Color::WHITE,
                            },
                            PaintCommand::Scrollbar {
                                track: op_paint::ClipRect {
                                    x: 40,
                                    y: 120,
                                    width: 10,
                                    height: 80,
                                },
                                thumb: op_paint::ClipRect {
                                    x: 41,
                                    y: 126,
                                    width: 8,
                                    height: 20,
                                },
                            },
                        ],
                    });
                    assert!(scrollbar_thumb_at_document_point(44, 130));
                    let down_y = TOOLBAR_HEIGHT + 130;
                    unsafe {
                        PostMessageW(
                            window.hwnd,
                            WM_LBUTTONDOWN,
                            1,
                            ((down_y as u32) << 16 | 44) as LPARAM,
                        );
                    }
                }
                9 => {
                    assert_eq!(event, NavigationEvent::ScrollbarDragStart { x: 44, y: 130 });
                    assert_eq!(unsafe { GetCapture() }, window.hwnd);
                    let move_y = TOOLBAR_HEIGHT + 190;
                    unsafe {
                        PostMessageW(
                            window.hwnd,
                            WM_MOUSEMOVE,
                            1,
                            ((move_y as u32) << 16 | 160) as LPARAM,
                        );
                    }
                }
                10 => {
                    assert_eq!(event, NavigationEvent::ScrollbarDragMove { y: 190 });
                    let up_y = TOOLBAR_HEIGHT + 190;
                    unsafe {
                        PostMessageW(
                            window.hwnd,
                            WM_LBUTTONUP,
                            0,
                            ((up_y as u32) << 16 | 160) as LPARAM,
                        );
                    }
                }
                11 => {
                    assert_eq!(event, NavigationEvent::ScrollbarDragEnd);
                    assert_ne!(unsafe { GetCapture() }, window.hwnd);
                    let click_y = TOOLBAR_HEIGHT + 40;
                    unsafe {
                        PostMessageW(
                            window.hwnd,
                            WM_LBUTTONUP,
                            0,
                            ((click_y as u32) << 16 | 15) as LPARAM,
                        );
                    }
                }
                12 => {
                    assert_eq!(event, NavigationEvent::Click { x: 15, y: 40 });
                    let down_y = TOOLBAR_HEIGHT + 130;
                    unsafe {
                        PostMessageW(
                            window.hwnd,
                            WM_LBUTTONDOWN,
                            1,
                            ((down_y as u32) << 16 | 44) as LPARAM,
                        );
                    }
                }
                13 => {
                    assert_eq!(event, NavigationEvent::ScrollbarDragStart { x: 44, y: 130 });
                    unsafe { ReleaseCapture() };
                }
                14 => {
                    assert_eq!(event, NavigationEvent::ScrollbarDragEnd);
                    assert!(!THUMB_DRAG_ACTIVE.load(Ordering::SeqCst));
                    window.close();
                }
                _ => panic!("unexpected extra UI event"),
            }
            step += 1;
        });
        done.send(()).unwrap();
        watchdog.join().unwrap();
        assert_eq!(exit, 0);
        assert_eq!(step, 15);
    }
}
