use std::{cell::OnceCell, pin::Pin};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, SIZE, TRUE, WPARAM},
        Graphics::Gdi::{GetWindowDC, InvalidateRect, COLOR_WINDOW, HBRUSH},
        UI::{
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW,
                LoadIconW, RegisterClassExW, SendMessageW, SetWindowLongPtrW, SetWindowPos,
                BS_DEFPUSHBUTTON, CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW, IDI_APPLICATION,
                SWP_FRAMECHANGED, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_SETFONT,
                WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_TABSTOP,
                WS_VISIBLE,
            },
        },
    },
};

use crate::util::{self, get_default_font, get_text_size_wrap, FontWrapper};

use super::{app::App, win_main::Instance};

const WINDOW_CLASS: PCWSTR = w!("myDialog001");

const DIALOG_CANCEL: i32 = 1;

#[derive(Debug)]
pub struct Dialog {
    pub p_app: *const App,
    h_dlg: HWND,
    h_text: HWND,
    h_input: HWND,
    h_button: HWND,
}

impl Dialog {
    pub fn register(instance: &Instance) {
        println!("Dialog::register");

        let wc_dialog = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(Self::window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance.h_instance,
            hIcon: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
            hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
            lpszMenuName: windows::core::PCWSTR(std::ptr::null()),
            lpszClassName: WINDOW_CLASS,
            hIconSm: unsafe { LoadIconW(None, IDI_APPLICATION).unwrap() },
        };

        println!("Registering dialog window class");
        unsafe { util::check_error_code(RegisterClassExW(&wc_dialog), 0).unwrap() };
    }

    #[allow(dead_code)]
    pub fn create(p_app: *const App) -> windows::core::Result<Pin<Box<Self>>> {
        println!("Dialog::create");

        let app = unsafe { p_app.as_ref() }.unwrap();

        let h_dlg = util::check_handle(unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                WINDOW_CLASS,
                w!("Select process"),
                WS_VISIBLE | WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800,
                300,
                app.hwnd,
                None,
                app.instance.h_instance,
                None,
            )
        })
        .unwrap();

        let h_text = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                INFO_TEXT,
                WS_VISIBLE | WS_CHILD,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                h_dlg,
                None,
                app.instance.h_instance,
                None,
            )
        })
        .unwrap();

        // TODO: 20px height + 2px border
        let h_input = util::check_handle(unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                h_dlg,
                None,
                app.instance.h_instance,
                None,
            )
        })
        .unwrap();

        let h_button = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Cancel"),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                h_dlg,
                HMENU(DIALOG_CANCEL as isize),
                app.instance.h_instance,
                None,
            )
        })
        .unwrap();

        // Need to pin because we are sending it to the window proc
        let dialog = Box::pin(Dialog {
            p_app,
            h_dlg,
            h_text,
            h_input,
            h_button,
        });

        // TODO: Somehow move the `Dialog` into the window proc??????
        // SEE:
        // - https://stackoverflow.com/questions/4341303/get-the-wndproc-for-windows-handle
        // - https://stackoverflow.com/questions/21369256/how-to-use-wndproc-as-a-class-function
        // - https://stackoverflow.com/questions/35178779/wndproc-as-class-method
        unsafe {
            SetWindowLongPtrW(
                h_dlg,
                GWLP_USERDATA,
                &*dialog.as_ref() as *const Dialog as isize,
            );
        }

        dialog.display_dialog();

        Ok(dialog)
    }

    fn apply_size(&self, sz_wnd: SIZE) {
        const BORDER_MARGIN: i32 = 10;

        // Calculate text
        let hdc = unsafe { GetWindowDC(self.h_text) };
        if hdc.is_invalid() {
            util::get_last_error().unwrap();
        }

        let text = unsafe { INFO_TEXT.as_wide() };
        let text_size = get_text_size_wrap(
            hdc,
            SIZE {
                cx: sz_wnd.cx, // - 2 * BORDER_MARGIN,
                cy: -1,
            },
            text,
        )
        .unwrap();

        /* TOP TO BOTTOM */

        let y = BORDER_MARGIN;
        let h = text_size.cy;

        unsafe {
            SetWindowPos(
                self.h_text,
                None,
                BORDER_MARGIN,
                y,
                (sz_wnd.cx - 2 * BORDER_MARGIN).min(text_size.cx),
                h,
                SWP_FRAMECHANGED,
            )
        }
        .unwrap();

        let y = y + h + 6;
        let h = 24;

        unsafe {
            SetWindowPos(
                self.h_input,
                None,
                BORDER_MARGIN,
                y,
                sz_wnd.cx - 2 * BORDER_MARGIN,
                h,
                SWP_FRAMECHANGED,
            )
        }
        .unwrap();

        /* BOTTOM */
        let w = 100;
        let h = 30;

        unsafe {
            SetWindowPos(
                self.h_button,
                None,
                sz_wnd.cx - w - BORDER_MARGIN,
                sz_wnd.cy - h - BORDER_MARGIN,
                w,
                h,
                SWP_FRAMECHANGED,
            )
        }
        .unwrap();
    }

    extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let pself = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog };
        let Some(self_) = (unsafe { pself.as_ref() }) else {
            println!("WARNING: reference to self (Dialog) is invalid (pself = {pself:p})");
            return LRESULT(0);
        };

        //println!("window_proc: pself == {:#?}", &self_);

        match msg {
            WM_SIZE => {
                let width = util::loword(lparam.0 as u32);
                let height = util::hiword(lparam.0 as u32);

                println!("WM_SIZE {{ width={width} height={height} }}");

                self_.apply_size(SIZE {
                    cx: width as i32,
                    cy: height as i32,
                });

                unsafe { InvalidateRect(hwnd, None, false) }.unwrap();
            }
            WM_CLOSE => unsafe { self_.destroy() },
            WM_COMMAND => match wparam.0 as i32 {
                DIALOG_CANCEL => unsafe { self_.destroy() },
                cmd => println!("{cmd}"),
            },
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }

        LRESULT(0)
    }

    #[allow(dead_code)]
    fn display_dialog(&self) {
        let dlg_rect = util::get_client_rect(self.h_dlg).unwrap();

        HF_DEFAULT.with(|cell_hf_default| {
            let hf_default = cell_hf_default.get_or_init(get_default_font);

            for handle in [self.h_dlg, self.h_button, self.h_text] {
                unsafe {
                    SendMessageW(
                        handle,
                        WM_SETFONT,
                        WPARAM(hf_default.as_ref().0 as usize),
                        LPARAM(TRUE.0 as isize),
                    )
                };
            }
        });

        let _ = unsafe { EnableWindow((*self.p_app).hwnd, false) };
        println!("window disabled");

        self.apply_size(SIZE {
            cx: dlg_rect.right - dlg_rect.left,
            cy: dlg_rect.bottom - dlg_rect.top,
        });
    }

    unsafe fn destroy(&self) {
        EnableWindow((*self.p_app).hwnd, true).unwrap();
        DestroyWindow(self.h_dlg).unwrap();
    }
}

impl Drop for Dialog {
    fn drop(&mut self) {
        println!("Dialog::drop");
        unsafe { self.destroy() };
    }
}

thread_local! {
    static HF_DEFAULT: OnceCell<FontWrapper> = OnceCell::new();
}

const INFO_TEXT: PCWSTR = w!("If you don't see the process you want to attach, try running with admin rights\nYou can also type in the process ID");
