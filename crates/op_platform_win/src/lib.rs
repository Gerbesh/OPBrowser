#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use op_paint::{Color, DisplayList, PaintCommand};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, EndPaint, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    FillRect, GetStockObject, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT, TextOutW, UpdateWindow, WHITE_BRUSH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW,
    GetMessageW, IDC_ARROW, LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SW_SHOW, ShowWindow,
    TranslateMessage, WM_DESTROY, WM_PAINT, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

static DISPLAY_LIST: OnceLock<RwLock<DisplayList>> = OnceLock::new();
static PAINTED_ONCE: AtomicBool = AtomicBool::new(false);

pub struct NativeBrowserWindow {
    hwnd: HWND,
}

impl NativeBrowserWindow {
    pub fn create(title: &str, display_list: DisplayList) -> Result<Self, String> {
        PAINTED_ONCE.store(false, Ordering::SeqCst);
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
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
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

        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
        }

        Ok(Self { hwnd })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn painted_once(&self) -> bool {
        PAINTED_ONCE.load(Ordering::SeqCst)
    }

    pub fn run_message_loop(&self) -> i32 {
        let mut message: MSG = unsafe { zeroed() };

        loop {
            let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
            if result == -1 {
                return 1;
            }
            if result == 0 {
                return message.wParam as i32;
            }

            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
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
