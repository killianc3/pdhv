#![windows_subsystem = "windows"]

use windows_sys::{
    w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{InvalidateRect, ScreenToClient, ValidateRect},
        UI::WindowsAndMessaging::{
            AdjustWindowRect, DefWindowProcW, DispatchMessageW, GetCursorPos,
            GetMessageW, GetPropW, PostQuitMessage, RemovePropW, SendMessageW, SetPropW,
            SetWindowPos, TranslateMessage, MINMAXINFO, MSG, SWP_NOACTIVATE, SWP_NOZORDER,
            WM_COMMAND, WM_DESTROY, WM_DPICHANGED, WM_GETMINMAXINFO, WM_PAINT, WM_SIZE,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use wgpu::SurfaceError;

use std::{cmp, mem, ptr, sync::mpsc, thread, time};

use pdhv::{graphic, menu, query, window};

struct App {
    menu: menu::Menu,
    graphic: graphic::Graphic,
    query: query::QueryV2,

    size: (i32, i32),
    dpi: u32,
}

fn main() {
    panic!("test");
    env_logger::init();

    #[allow(clippy::missing_safety_doc)]
    unsafe {
        let window = window::Window::new(Some(wnd_proc));

        let mut menu = menu::Menu::new(window.hwnd);
        let graphic = pollster::block_on(graphic::Graphic::new(&window));
        let query = query::QueryV2::new(window.hwnd, &mut menu);

        let mut app = App {
            menu,
            graphic,
            query,

            size: window.get_size(),
            dpi: window.get_dpi(),
        };

        assert!(SetPropW(window.hwnd, w!("app"), &mut app as *mut App as _) != 0);

        let (_tx, rx) = mpsc::channel::<()>();
        thread::spawn(move || loop {
            if let Err(mpsc::TryRecvError::Disconnected) = rx.try_recv() {
                break;
            }

            SendMessageW(window.hwnd, WM_PAINT, 0, 0);
            thread::sleep(time::Duration::from_millis(33));
        });

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        app.query.close();
        RemovePropW(window.hwnd, w!("app"));
    }
}

#[allow(clippy::missing_safety_doc)]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let papp = happ as *mut App;
                match wparam as u16 as isize {
                    menu::IDM_COUNTER_NEW => {
                        (*papp).query.add_counter(hwnd, &mut (*papp).menu, None)
                    }
                    menu::IDM_COUNTER_REMOVE_ALL => {
                        (*papp).query.remove_all_counter(&mut (*papp).menu)
                    }
                    menu::IDM_LOG_START => (*papp).query.start_logging(&mut (*papp).menu, hwnd),
                    menu::IDM_LOG_STOP => (*papp).query.stop_logging(&mut (*papp).menu),
                    id if menu::IDM_REMOVE_RANGE.contains(&id) => (*papp)
                        .query
                        .remove_counter(id - menu::IDM_REMOVE_RANGE.start, &mut (*papp).menu),
                    _ => (),
                }

                0
            }
        },
        WM_SIZE => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let papp = happ as *mut App;

                (*papp)
                    .graphic
                    .resize((lparam as u16 as _, (lparam >> 16) as u16 as _));
                (*papp).size = (lparam as u16 as _, (lparam >> 16) as u16 as _);

                0
            }
        },
        WM_PAINT => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let mut pt = mem::zeroed::<POINT>();
                GetCursorPos(&mut pt);
                ScreenToClient(hwnd, &mut pt);

                let papp = happ as *mut App;
                (*papp)
                    .graphic
                    .update(&(*papp).query, (*papp).dpi, pt.x, pt.y);

                if let Err(SurfaceError::OutOfMemory) = (*papp).graphic.render() {
                    SendMessageW(hwnd, WM_DESTROY, 0, 0);
                }
                ValidateRect(hwnd, ptr::null());

                0
            }
        },
        WM_GETMINMAXINFO => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let papp = happ as *mut App;
                let nb_box = (*papp).query.counters.len();
                let nb_box_x = cmp::max(
                    1,
                    cmp::min(
                        nb_box,
                        (*papp).size.0 as usize
                            / (400.0 * ((*papp).dpi as f32 / graphic::BASE_DPI)) as usize,
                    ),
                );
                let nb_box_y = (nb_box as f32 / nb_box_x as f32).ceil() as usize;

                let mut rect = RECT {
                    top: 0,
                    left: 0,
                    bottom: (graphic::MIN_BOX_HEIGHT as f32
                        * ((*papp).dpi as f32 / graphic::BASE_DPI)
                        * nb_box_y as f32) as i32,
                    right: (graphic::MIN_BOX_WIDTH as f32
                        * ((*papp).dpi as f32 / graphic::BASE_DPI))
                        as i32,
                };
                AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, true.into());

                let minmaxinfo = lparam as *mut MINMAXINFO;
                (*minmaxinfo).ptMinTrackSize.x = rect.right - rect.left;
                (*minmaxinfo).ptMinTrackSize.y = rect.bottom - rect.top;

                0
            }
        },
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_DPICHANGED => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let papp = happ as *mut App;
                (*papp).dpi = wparam as u16 as _;
                let prc_new_window = lparam as *const RECT;
                (*papp).size = (
                    (*prc_new_window).right - (*prc_new_window).left,
                    (*prc_new_window).bottom - (*prc_new_window).top,
                );
                SetWindowPos(
                    hwnd,
                    0,
                    (*prc_new_window).left,
                    (*prc_new_window).top,
                    (*prc_new_window).right - (*prc_new_window).left,
                    (*prc_new_window).bottom - (*prc_new_window).top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );

                0
            }
        },
        query::WM_UPDATE_QUERY => match GetPropW(hwnd, w!("app")) {
            0 => {
                //SendMessageW(hwnd, WM_DESTROY, 0, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            happ => {
                let papp = happ as *mut App;
                (*papp).query.update(&mut (*papp).menu);
                InvalidateRect(hwnd, ptr::null(), false.into());

                0
            }
        },
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
