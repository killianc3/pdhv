use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle,
    WindowsDisplayHandle,
};

use windows_sys::{
    w,
    Win32::{
        Foundation::{HINSTANCE, HWND, RECT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{
                GetDpiForWindow, SetProcessDpiAwarenessContext,
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            },
            WindowsAndMessaging::{
                CreateWindowExW, GetClientRect, GetForegroundWindow, LoadImageW, RegisterClassW,
                ShowWindow, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, IMAGE_CURSOR,
                IMAGE_ICON, LR_LOADFROMFILE, LR_SHARED, WNDCLASSW, WNDPROC, WS_OVERLAPPEDWINDOW,
                WS_VISIBLE,
            },
        },
    },
};

use std::{mem, ptr};

pub struct Window {
    hinst: HINSTANCE,
    pub hwnd: HWND,
}

impl Window {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(wnd_proc: WNDPROC) -> Self {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinst = GetModuleHandleW(ptr::null_mut());
        assert!(hinst != 0);

        let hicon = LoadImageW(
            hinst,
            w!("icon.ico"),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE,
        );
        let hcursor = LoadImageW(0, IDC_ARROW, IMAGE_CURSOR, 0, 0, LR_SHARED);

        let mut wnd_class_w: WNDCLASSW = mem::zeroed();
        wnd_class_w.lpfnWndProc = wnd_proc;
        wnd_class_w.hInstance = hinst;
        wnd_class_w.style = CS_VREDRAW | CS_HREDRAW;
        wnd_class_w.lpszClassName = w!("pdhv");
        wnd_class_w.hIcon = hicon;
        wnd_class_w.hCursor = hcursor;

        assert!(RegisterClassW(&wnd_class_w) != 0);

        let hwnd = CreateWindowExW(
            0,
            w!("pdhv"),
            w!("pdhv"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            hinst,
            ptr::null_mut(),
        );
        assert!(hwnd != 0);

        let _ = ShowWindow(hwnd, true.into());

        Self { hinst, hwnd }
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_size(&self) -> (i32, i32) {
        let mut rc: RECT = mem::zeroed();
        GetClientRect(self.hwnd, &mut rc);

        (rc.right - rc.left, rc.bottom - rc.top)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_dpi(&self) -> u32 {
        GetDpiForWindow(self.hwnd)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn is_active(&self) -> bool {
        self.hwnd == GetForegroundWindow()
    }
}

#[allow(clippy::missing_safety_doc)]
unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut window_handle = Win32WindowHandle::empty();
        window_handle.hwnd = self.hwnd as *mut _;
        window_handle.hinstance = self.hinst as *mut _;
        RawWindowHandle::Win32(window_handle)
    }
}

#[allow(clippy::missing_safety_doc)]
unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Windows(WindowsDisplayHandle::empty())
    }
}
