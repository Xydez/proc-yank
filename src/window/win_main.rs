use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{GetLastError, BOOL, HMODULE, HWND, LPARAM, LRESULT, TRUE, WPARAM},
        Graphics::Gdi::{
            CreateFontIndirectW, DeleteObject, UpdateWindow, COLOR_WINDOW, HBRUSH, HFONT,
        },
        UI::{
            Controls::{InitCommonControlsEx, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX},
            WindowsAndMessaging::{
                AppendMenuW, CreateMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
                DispatchMessageW, EnumChildWindows, GetMessageW, LoadCursorW, LoadIconW,
                MessageBoxW, PostQuitMessage, RegisterClassExW, SendMessageW, SetMenu, ShowWindow,
                TranslateMessage, CW_USEDEFAULT, IDC_ARROW, IDI_APPLICATION, MB_ICONEXCLAMATION,
                MB_OK, MF_POPUP, MF_STRING, MSG, SW_SHOW, WINDOW_STYLE, WM_CLOSE, WM_COMMAND,
                WM_CREATE, WM_DESTROY, WM_SETFONT, WNDCLASSEXW, WS_EX_WINDOWEDGE,
                WS_OVERLAPPEDWINDOW, WS_THICKFRAME,
            },
        },
    },
};

use crate::{
    id::{FILE_MENU_EXIT, FILE_MENU_NEW},
    util::get_non_client_metrics,
    window::dialog,
};

const WINDOW_CLASS: PCWSTR = w!("myWindowClass");

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
            add_menus(hwnd).unwrap();
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }

    return windows::Win32::Foundation::LRESULT(0);
}

unsafe extern "system" fn enum_child_proc(h_wnd: HWND, lparam: LPARAM) -> BOOL {
    let hf_default: HFONT = HFONT(lparam.0);
    SendMessageW(
        h_wnd,
        WM_SETFONT,
        WPARAM(hf_default.0 as usize),
        LPARAM(TRUE.0 as isize),
    );
    return TRUE;
}

pub unsafe fn win_main(
    h_instance: HMODULE,
    _h_prev_instance: HMODULE,
    _p_cmd_line: PCWSTR,
    n_cmd_show: u32,
) {
    /* 0. Initialize common controls */
    InitCommonControlsEx(&INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES,
        ..Default::default()
    })
    .unwrap();

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
        WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 ^ WS_THICKFRAME.0),
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        800,
        600,
        None,
        None,
        h_instance,
        None,
    );

    // Retrieve the default font
    let ncmetrics = get_non_client_metrics();
    let hf_default = CreateFontIndirectW(&ncmetrics.lfMessageFont);
    println!(
        "Default font: {}",
        HSTRING::from_wide(&ncmetrics.lfMessageFont.lfFaceName).unwrap()
    );

    println!("Showing window");
    dbg!(n_cmd_show);
    let _ = ShowWindow(
        hwnd, SW_SHOW, //windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD(n_cmd_show),
    );

    let _ = EnumChildWindows(hwnd, Some(enum_child_proc), LPARAM(hf_default.0));

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

    DeleteObject(hf_default).unwrap();
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
