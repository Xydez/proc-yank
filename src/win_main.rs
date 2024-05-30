use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM},
        Graphics::Gdi::{UpdateWindow, COLOR_WINDOW, HBRUSH},
        System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassExW, ShowWindow,
            TranslateMessage, CW_USEDEFAULT, IDC_ARROW, IDI_APPLICATION, MB_ICONEXCLAMATION, MB_OK,
            MSG, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSEXW,
            WS_EX_WINDOWEDGE, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::{
    dialog,
    gui::{add_controls, add_gui},
    id::{FILE_MENU_EXIT, FILE_MENU_NEW},
    util::get_last_error,
};

/*
#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("windows error: {0}")]
    WinError(#[from] ::windows::core::Error),
}
*/

const WINDOW_CLASS: PCWSTR = w!("myWindowClass");

fn get_instance_name() -> ::windows::core::Result<HSTRING> {
    let h_instance = unsafe { GetModuleHandleW(None)? };

    let mut sz_file_name = Box::new([0u16; MAX_PATH as usize]);
    let length = unsafe { GetModuleFileNameW(h_instance, &mut *sz_file_name) };

    if length == 0 {
        unsafe { get_last_error() }?;
    }

    let str = unsafe { ::windows::core::PCWSTR::from_raw(sz_file_name.as_ptr()).to_hstring()? };

    println!("{}", str);

    Ok(str)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            println!("Destroting window");
            DestroyWindow(hwnd).unwrap();
        }
        WM_DESTROY => {
            PostQuitMessage(0);
        }
        WM_COMMAND => match wparam.0 as u32 {
            FILE_MENU_NEW => {
                println!("display_dialog");
                dialog::display_dialog(hwnd)
            }
            FILE_MENU_EXIT => {
                PostQuitMessage(0);
            }
            _ => (),
        },
        WM_CREATE => {
            add_gui(hwnd);
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }

    return windows::Win32::Foundation::LRESULT(0);
}

pub unsafe fn win_main(
    h_instance: HMODULE,
    _h_prev_instance: HMODULE,
    _p_cmd_line: PCWSTR,
    n_cmd_show: u32,
) {
    /* 1. Register classes */
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance.into(),
        hIcon: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
        lpszMenuName: windows::core::PCWSTR(std::ptr::null()),
        lpszClassName: WINDOW_CLASS,
        hIconSm: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
        ..Default::default()
    };

    println!("Registering window class");
    if RegisterClassExW(&wc) == 0 {
        let error = GetLastError();
        dbg!(&error);

        MessageBoxW(
            None,
            w!("Failed to register window"),
            w!("Error"),
            MB_ICONEXCLAMATION | MB_OK,
        );
        return;
    }

    dialog::register_dialog_class(h_instance.into()).unwrap();

    /* 2. Create windows */

    println!("Creating window");
    let hwnd = CreateWindowExW(
        WS_EX_WINDOWEDGE, // WS_EX_CLIENTEDGE
        WINDOW_CLASS,
        w!("Window Title"),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        800,
        600,
        None,
        None,
        h_instance,
        None,
    );

    println!("Showing window");
    dbg!(n_cmd_show);
    let _ = ShowWindow(
        hwnd, SW_SHOW, //windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD(n_cmd_show),
    );
    println!("Updating window");
    let _ = UpdateWindow(hwnd);

    // Step 3: The Message Loop
    let mut msg = MSG::default();

    println!("Running window loop");
    while GetMessageW(&mut msg, None, 0, 0).ok().is_ok() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    println!("Exiting");
}
