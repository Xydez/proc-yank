use std::cell::{OnceCell, RefCell};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, TRUE, WPARAM},
        Graphics::Gdi::{GetWindowDC, InvalidateRect, COLOR_WINDOW, HBRUSH},
        UI::{
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
                LoadCursorW, LoadIconW, RegisterClassExW, SendMessageW, SetWindowPos,
                BS_DEFPUSHBUTTON, CW_USEDEFAULT, GWLP_HINSTANCE, HMENU, IDC_ARROW, IDI_APPLICATION,
                SWP_FRAMECHANGED, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_SETFONT,
                WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_TABSTOP,
                WS_VISIBLE,
            },
        },
    },
};

use crate::util::{self, get_default_font, get_last_error, get_text_size_wrap, FontWrapper};

const WNDCLASS_DIALOG: PCWSTR = w!("myDialog001");

const DIALOG_CANCEL: i32 = 1;

#[derive(Debug)]
struct Data {
    hwnd_master: HWND,
    hdlg: HWND,
    htext: HWND,
    hinput: HWND,
    hbutton: HWND,
}

thread_local! {
    static DATA: RefCell<Option<Data>> = RefCell::new(None);
    static HF_DEFAULT: OnceCell<FontWrapper> = OnceCell::new();
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            let width = util::loword(lparam.0 as u32);
            let height = util::hiword(lparam.0 as u32);

            println!("WM_SIZE {{ width={width} height={height} }}");

            DATA.with(|data_cell| {
                let data_ref = data_cell.borrow();
                let data = data_ref.as_ref();

                if let Some(data) = data {
                    apply_size(
                        &data,
                        SIZE {
                            cx: width as i32,
                            cy: height as i32,
                        },
                    );

                    InvalidateRect(hwnd, None, false).unwrap();
                }
            });
        }
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
    DATA.with(|data_cell| {
        let data_ref = data_cell.borrow();
        let data = data_ref.as_ref().unwrap();

        EnableWindow(data.hwnd_master, true).unwrap();

        drop(data_ref);

        data_cell.swap(&RefCell::new(None));
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

const INFO_TEXT: PCWSTR = w!("If you don't see the process you want to attach, try running with admin rights\nYou can also type in the process ID");

pub unsafe fn display_dialog(hwnd: HWND) {
    let hinstance = HINSTANCE(GetWindowLongPtrW(hwnd, GWLP_HINSTANCE));

    let hdlg = util::check_handle(CreateWindowExW(
        WS_EX_CLIENTEDGE,
        WNDCLASS_DIALOG,
        w!("Select process"),
        WS_VISIBLE | WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        800,
        300,
        hwnd,
        None,
        hinstance,
        None,
    ))
    .unwrap();

    let mut dlg_rect = RECT::default();
    GetClientRect(hdlg, &mut dlg_rect).unwrap();

    let h_text = util::check_handle(CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        INFO_TEXT,
        WS_VISIBLE | WS_CHILD,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        hdlg,
        None,
        hinstance,
        None,
    ))
    .unwrap();

    // TODO: 20px height + 2px border
    let _h_input = util::check_handle(CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        WS_CHILD | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        hdlg,
        None,
        hinstance,
        None,
    ))
    .unwrap();

    let h_button = util::check_handle(CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Cancel"),
        WS_TABSTOP | WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        hdlg,
        HMENU(DIALOG_CANCEL as isize),
        hinstance,
        None,
    ))
    .unwrap();

    HF_DEFAULT.with(|cell_hf_default| {
        let hf_default = cell_hf_default.get_or_init(|| get_default_font());

        for handle in [hdlg, h_button, h_text] {
            SendMessageW(
                handle,
                WM_SETFONT,
                WPARAM(hf_default.as_ref().0 as usize),
                LPARAM(TRUE.0 as isize),
            );
        }
    });

    let _ = EnableWindow(hwnd, false);
    println!("window disabled");

    // Update references
    DATA.with(|data_cell| {
        data_cell.swap(&RefCell::new(Some(Data {
            hwnd_master: hwnd,
            hdlg,
            htext: h_text,
            hinput: _h_input,
            hbutton: h_button,
        })));
    });

    DATA.with(|data_cell| {
        let data_ref = data_cell.borrow();
        let data = data_ref.as_ref().unwrap();

        apply_size(
            &data,
            SIZE {
                cx: dlg_rect.right - dlg_rect.left as i32,
                cy: dlg_rect.bottom - dlg_rect.top as i32,
            },
        );
    });
}

unsafe fn apply_size(data: &Data, sz_wnd: SIZE) {
    const BORDER_MARGIN: i32 = 10;

    // Calculate text
    let hdc = GetWindowDC(data.htext);
    if hdc.is_invalid() {
        util::get_last_error().unwrap();
    }

    let text_size = get_text_size_wrap(
        hdc,
        SIZE {
            cx: sz_wnd.cx, // - 2 * BORDER_MARGIN,
            cy: -1,
        },
        INFO_TEXT.as_wide(),
    )
    .unwrap();

    /* TOP TO BOTTOM */

    let y = BORDER_MARGIN;
    let h = text_size.cy;

    SetWindowPos(
        data.htext,
        None,
        BORDER_MARGIN,
        y,
        (sz_wnd.cx - 2 * BORDER_MARGIN).min(text_size.cx),
        h,
        SWP_FRAMECHANGED,
    )
    .unwrap();

    let y = y + h + 6;
    let h = 24;

    SetWindowPos(
        data.hinput,
        None,
        BORDER_MARGIN,
        y,
        sz_wnd.cx - 2 * BORDER_MARGIN,
        h,
        SWP_FRAMECHANGED,
    )
    .unwrap();

    /* BOTTOM */
    let w = 100;
    let h = 30;

    SetWindowPos(
        data.hbutton,
        None,
        sz_wnd.cx - w - BORDER_MARGIN,
        sz_wnd.cy - h - BORDER_MARGIN,
        w,
        h,
        SWP_FRAMECHANGED,
    )
    .unwrap();
}
