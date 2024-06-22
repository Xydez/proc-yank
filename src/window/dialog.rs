use std::sync::Arc;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, SIZE, TRUE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetWindowDC, InvalidateRect, COLOR_WINDOW, HBRUSH, HFONT},
        UI::{
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindow, GetWindowLongPtrW,
                LoadCursorW, LoadIconW, RegisterClassExW, SendMessageW, SetWindowLongPtrW,
                SetWindowPos, BS_DEFPUSHBUTTON, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA,
                GW_OWNER, HMENU, IDC_ARROW, IDI_APPLICATION, SWP_FRAMECHANGED, WINDOW_EX_STYLE,
                WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_SETFONT, WM_SIZE,
                WNDCLASSEXW, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_TABSTOP,
                WS_VISIBLE,
            },
        },
    },
};

use crate::{
    id::IDC_DIALOG_CANCEL,
    string::INFO_TEXT,
    util::{self, get_default_font, get_text_size_wrap},
};

use super::{app::App, win_main::Instance};

const WINDOW_CLASS: PCWSTR = w!("myDialog001");

#[derive(Debug)]
pub struct Dialog {
    #[allow(dead_code)]
    hwnd: HWND,
    hwnd_text: HWND,
    hwnd_input: HWND,
    hwnd_button: HWND,
    hf_default: HFONT,
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

    pub fn create(app: &App) -> windows::core::Result<Arc<Dialog>> {
        println!("Dialog::create app.hwnd={}", app.hwnd.0);

        let hwnd = util::check_handle(unsafe {
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
                Some(Arc::into_raw(Arc::clone(&app.instance)) as *const std::ffi::c_void),
            )
        })
        .unwrap();

        let dialog = unsafe {
            // Clone the Arc in GWLP_USERDATA safely
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;

            assert!(
                !ptr.is_null(),
                "Dialog pointer should not be null between WM_CREATE and WM_DESTROY"
            );

            Arc::increment_strong_count(ptr);
            Arc::from_raw(ptr)
        };

        Ok(dialog)
    }

    // pub fn show(&self) -> ::windows::core::Result<()> {
    //     unimplemented!("Do we really need this?")
    // }

    fn create_window(instance: Arc<Instance>, hwnd: HWND) -> ::windows::core::Result<Dialog> {
        let hwnd_text = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                INFO_TEXT,
                WS_VISIBLE | WS_CHILD,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                None,
                instance.h_instance,
                None,
            )
        })
        .unwrap();

        // TODO: 20px height + 2px border
        let hwnd_input = util::check_handle(unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                None,
                instance.h_instance,
                None,
            )
        })
        .unwrap();

        let hwnd_button = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Cancel"),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                HMENU(IDC_DIALOG_CANCEL as isize),
                instance.h_instance,
                None,
            )
        })
        .unwrap();

        let hf_default = get_default_font();

        for handle in [hwnd, hwnd_text, hwnd_input, hwnd_button] {
            unsafe {
                SendMessageW(
                    handle,
                    WM_SETFONT,
                    WPARAM(hf_default.0 as usize),
                    LPARAM(TRUE.0 as isize),
                )
            };
        }

        let dlg_rect = util::get_client_rect(hwnd).unwrap();

        let dialog = Dialog {
            hwnd,
            hwnd_text,
            hwnd_input,
            hwnd_button,
            hf_default,
        };

        dialog.apply_size(SIZE {
            cx: dlg_rect.right - dlg_rect.left,
            cy: dlg_rect.bottom - dlg_rect.top,
        });

        Ok(dialog)
    }

    fn apply_size(&self, sz_wnd: SIZE) {
        const BORDER_MARGIN: i32 = 10;

        // Calculate text
        let hdc = unsafe { GetWindowDC(self.hwnd_text) };
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
                self.hwnd_text,
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
                self.hwnd_input,
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
                self.hwnd_button,
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

    /// The destroy procedure for the window
    unsafe fn destroy(hwnd: HWND) {
        // Re-enable the parent
        // See: https://web.archive.org/web/20100627175601/http://blogs.msdn.com/b/oldnewthing/archive/2004/02/27/81155.aspx
        let parent = util::check(|| GetWindow(hwnd, GW_OWNER)).unwrap();
        let _ = util::check(|| EnableWindow(parent, true)).unwrap();

        // Destroy the window
        DestroyWindow(hwnd).unwrap();
    }

    extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                /* Create the window */
                // Retrieve the instance from the CreateWindowEx lpparam
                let create_params = unsafe { *(lparam.0 as *const CREATESTRUCTW) };

                let instance =
                    unsafe { Arc::from_raw(create_params.lpCreateParams as *const Instance) };

                // Create the window and set the pointer
                let dialog =
                    Arc::new(Self::create_window(instance, hwnd).expect("Failed to create window"));

                unsafe {
                    // We need to make it a Arc<Dialog> to make it stay on the heap until WM_DESTROY
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::into_raw(dialog) as isize);
                }

                // Disable the parent
                unsafe {
                    let _ = EnableWindow(create_params.hwndParent, false);
                }
            }
            WM_CLOSE => {
                unsafe { Self::destroy(hwnd) };
            }
            // WM_DESTROY is used to free the allocated memory object associated with the window.
            WM_DESTROY => {
                println!("Dialog WM_DESTROY");

                // Retrieve the `Arc`
                let app = unsafe {
                    Arc::from_raw(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog)
                };

                /* Delete objects */
                unsafe {
                    DeleteObject(app.hf_default).unwrap();
                }

                /* Drop the Arc */

                // Drop the arc stored in GWLP_USERDATA. This does not mean that other `Arc`s to the App are dropped.
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, std::ptr::null::<Dialog>() as isize)
                };

                drop(app);

                // N.B.: Child windows are destroyed after WM_DESTROY but before WM_NCDESTROY
            }
            WM_SIZE => {
                let width = util::loword(lparam.0 as u32);
                let height = util::hiword(lparam.0 as u32);

                println!("WM_SIZE {{ width={width} height={height} }}");

                let self_ = unsafe {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;

                    assert!(
                        !ptr.is_null(),
                        "WM_SIZE cannot be called before WM_CREATE or after WM_DESTROY"
                    );

                    &*ptr
                };

                self_.apply_size(SIZE {
                    cx: width as i32,
                    cy: height as i32,
                });

                unsafe { InvalidateRect(hwnd, None, false) }.unwrap();
            }
            WM_COMMAND =>
            {
                #[allow(clippy::single_match)]
                match util::loword(wparam.0 as u32) {
                    IDC_DIALOG_CANCEL => unsafe { Self::destroy(hwnd) },
                    _ => (),
                }
            }
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }

        LRESULT(0)
    }
}
