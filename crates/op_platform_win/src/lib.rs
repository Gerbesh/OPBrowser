#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};

use op_paint::{Color, DisplayList, PaintCommand};
mod raster;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, EndPaint, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    FillRect, GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, ScreenToClient, SelectObject, SetBkMode, SetTextColor,
    SetViewportOrgEx, TRANSPARENT, TextOutW, UpdateWindow, WHITE_BRUSH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, SetFocus, VK_CONTROL, VK_F5, VK_L, VK_RETURN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static DISPLAY_LIST: OnceLock<RwLock<DisplayList>> = OnceLock::new();
static PAINTED_ONCE: AtomicBool = AtomicBool::new(false);
static PAINTED_IMAGES: AtomicUsize = AtomicUsize::new(0);
static SCROLL_Y: AtomicI32 = AtomicI32::new(0);
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
const NAVIGATION_COMMAND: u32 = WM_APP + 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationEvent {
    Navigate(String),
    FollowLink(String),
    Click { x: i32, y: i32 },
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
            WM_LBUTTONUP if message.hwnd == self.hwnd => {
                let x = (message.lParam as u16 as i16) as i32;
                let y = ((message.lParam >> 16) as u16 as i16) as i32;
                if let Some(href) = self.link_at_client_point(x, y) {
                    Some(NavigationEvent::FollowLink(href))
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
                }
            }
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
            WM_TIMER if message.hwnd == self.hwnd && message.wParam == RESIZE_TIMER => {
                unsafe {
                    KillTimer(self.hwnd, RESIZE_TIMER);
                }
                Some(NavigationEvent::Resize)
            }
            _ => None,
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

fn link_at_client_point(hwnd: HWND, x: i32, y: i32) -> Option<String> {
    let mut client: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    if x < 0 || x >= client.right || y < TOOLBAR_HEIGHT || y >= client.bottom {
        return None;
    }
    let document_y = y - TOOLBAR_HEIGHT + SCROLL_Y.load(Ordering::SeqCst);
    LINK_REGIONS
        .get()?
        .read()
        .ok()?
        .iter()
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
            let max_scroll = max_scroll(hwnd);
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
                    assert_eq!(event, NavigationEvent::FollowLink("../target".into()));
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
                    assert_eq!(event, NavigationEvent::FollowLink("../image".into()));
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
                    window.close();
                }
                _ => panic!("unexpected extra UI event"),
            }
            step += 1;
        });
        done.send(()).unwrap();
        watchdog.join().unwrap();
        assert_eq!(exit, 0);
        assert_eq!(step, 9);
    }
}
