use std::cell::Cell;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{COLOR_WINDOW, HBRUSH},
        UI::{
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW,
                LoadIconW, RegisterClassExW, BS_DEFPUSHBUTTON, CW_USEDEFAULT, GWLP_HINSTANCE,
                HMENU, IDC_ARROW, IDI_APPLICATION, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WNDCLASSEXW,
                WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
};

use crate::util::{self, get_last_error};

const WNDCLASS_DIALOG: PCWSTR = w!("myDialog001");

const DIALOG_CANCEL: i32 = 1;

thread_local! {
    static HWND_MASTER: Cell<Option<HWND>> = Cell::new(None);
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => destroy(hwnd),
        WM_COMMAND => match wparam.0 as i32 {
            DIALOG_CANCEL => destroy(hwnd),
            cmd => println!("{cmd}"),
        },
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }

    return windows::Win32::Foundation::LRESULT(0);
}

pub unsafe fn destroy(hwnd: HWND) {
    HWND_MASTER.with(|hwnd_master| {
        EnableWindow(hwnd_master.get().unwrap(), true).unwrap();
    });

    DestroyWindow(hwnd).unwrap();
}

pub unsafe fn register_dialog_class(h_instance: HINSTANCE) -> ::windows::core::Result<()> {
    let wc_dialog = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
        lpfnWndProc: Some(dialog_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance.into(),
        hIcon: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
        lpszMenuName: windows::core::PCWSTR(std::ptr::null()),
        lpszClassName: WNDCLASS_DIALOG,
        hIconSm: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
        ..Default::default()
    };

    if RegisterClassExW(&wc_dialog) == 0 {
        get_last_error()?;
    }

    Ok(())
}

pub unsafe fn display_dialog(hwnd: HWND) {
    let hdlg = util::check_handle(CreateWindowExW(
        WS_EX_CLIENTEDGE,
        WNDCLASS_DIALOG,
        w!("Select process"),
        WS_VISIBLE | WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        400,
        300,
        hwnd,
        None,
        HINSTANCE(GetWindowLongPtrW(hwnd, GWLP_HINSTANCE)),
        None,
    ))
    .unwrap();

    util::check_handle(CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("BUTTON"),
        w!("Cancel"),
        WS_TABSTOP | WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        20,
        20,
        100,
        40,
        hdlg,
        HMENU(DIALOG_CANCEL as isize),
        HINSTANCE(GetWindowLongPtrW(hwnd, GWLP_HINSTANCE)),
        None,
    ))
    .unwrap();

    HWND_MASTER.with(|hwnd_master| {
        hwnd_master.swap(&Cell::new(Some(hwnd)));
    });

    let _ = EnableWindow(hwnd, false);
    println!("window disabled");
}
