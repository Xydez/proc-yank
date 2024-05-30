use windows::{
    core::w,
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            AppendMenuW, CreateMenu, CreateWindowExW, MessageBoxW, SetMenu, MB_ICONEXCLAMATION,
            MB_OK, MF_POPUP, MF_STRING, WS_CHILD, WS_EX_CLIENTEDGE, WS_VISIBLE,
        },
    },
};

use crate::id::{FILE_MENU_EXIT, FILE_MENU_NEW};

pub unsafe fn add_gui(hwnd: HWND) {
    add_menus(hwnd).unwrap();
    add_controls(hwnd);
}

pub unsafe fn add_menus(hwnd: HWND) -> ::windows::core::Result<()> {
    let h_menu = CreateMenu()?;

    let h_file_menu = CreateMenu()?;

    AppendMenuW(
        h_file_menu,
        MF_STRING,
        FILE_MENU_NEW as usize,
        w!("Attach to program"),
    )?;
    AppendMenuW(h_file_menu, MF_STRING, FILE_MENU_EXIT as usize, w!("Exit"))?;

    AppendMenuW(h_menu, MF_POPUP, h_file_menu.0 as usize, w!("File"))?;

    SetMenu(hwnd, h_menu)?;

    Ok(())
}

pub unsafe fn add_controls(hwnd: HWND) {
    /*
    CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("Static"),
        w!("Fuckyou"),
        WS_VISIBLE | WS_CHILD,
        50,
        50,
        100,
        40,
        hwnd,
        None,
        None,
        None,
    );
    */
}
