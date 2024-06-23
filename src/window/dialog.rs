use std::{collections::HashMap, sync::Arc};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, SIZE, TRUE, WPARAM},
        Graphics::Gdi::{DeleteObject, GetWindowDC, InvalidateRect, COLOR_WINDOW, HBRUSH, HFONT},
        UI::{
            Controls::{
                ImageList_Create, ImageList_ReplaceIcon, ILC_COLOR32, LVCFMT_LEFT, LVCF_FMT,
                LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_IMAGE, LVIF_TEXT, LVITEMW,
                LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMW,
                LVN_GETDISPINFOW, LVN_INSERTITEM, LVSIL_SMALL, LVS_AUTOARRANGE,
                LVS_EX_FULLROWSELECT, LVS_REPORT, NMHDR, NMITEMACTIVATE, NM_CLICK, WC_BUTTONW,
                WC_EDITW, WC_LISTVIEWW, WC_STATICW,
            },
            Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindow, GetWindowLongPtrW,
                LoadCursorW, LoadIconW, RegisterClassExW, SendMessageW, SetWindowLongPtrW,
                SetWindowPos, BS_DEFPUSHBUTTON, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA,
                GW_OWNER, HMENU, IDC_ARROW, IDI_APPLICATION, MINMAXINFO, SWP_FRAMECHANGED,
                WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY,
                WM_GETMINMAXINFO, WM_NOTIFY, WM_SETFONT, WM_SIZE, WNDCLASSEXW, WS_CHILD,
                WS_DISABLED, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
};

use crate::{
    id::{IDC_DIALOG_CANCEL, IDC_DIALOG_OK},
    memory::{FileInfoField, Process, ProcessArchitecture, ProcessSnapshot},
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
    hwnd_listview: HWND,
    hwnd_button_cancel: HWND,
    hwnd_button_ok: HWND,
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
                640,
                480,
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

    /// Create all the children within the window
    ///
    /// Note: See [apply_size](Self::apply_size) for positioning the children
    fn create_window(instance: Arc<Instance>, hwnd: HWND) -> ::windows::core::Result<Dialog> {
        let hwnd_text = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WC_STATICW,
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
                WC_EDITW,
                None,
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

        let hwnd_listview = util::check_handle(unsafe {
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                WC_LISTVIEWW,
                None,
                WS_CHILD
                    | WS_VISIBLE
                    | WINDOW_STYLE(
                        LVS_REPORT | LVS_AUTOARRANGE, // /* todo: remove? */ LVS_OWNERDATA |
                    ), // | LVS_EDITLABELS | LVS_AUTOARRANGE
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                HMENU(crate::id::ID_LISTVIEW as isize), // TODO: Isn't this for dialogs?
                instance.h_instance,
                None,
            )
        })
        .unwrap();

        let hwnd_button_cancel = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WC_BUTTONW,
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

        let hwnd_button_ok = util::check_handle(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WC_BUTTONW,
                w!("Ok"),
                WS_TABSTOP
                    | WS_VISIBLE
                    | WS_CHILD
                    | WS_DISABLED
                    | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                HMENU(IDC_DIALOG_OK as isize),
                instance.h_instance,
                None,
            )
        })
        .unwrap();

        let hf_default = get_default_font();

        for handle in [
            hwnd,
            hwnd_text,
            hwnd_input,
            hwnd_button_cancel,
            hwnd_button_ok,
        ] {
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
            hwnd_listview,
            hwnd_button_cancel,
            hwnd_button_ok,
            hf_default,
        };

        dialog.apply_size(SIZE {
            cx: dlg_rect.right - dlg_rect.left,
            cy: dlg_rect.bottom - dlg_rect.top,
        });

        Ok(dialog)
    }

    fn apply_size(&self, sz_wnd: SIZE) {
        // See: https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-layout#recommended-sizing-and-spacing
        const BORDER_MARGIN: i32 = 10;
        const SPACER: i32 = 6;
        const INPUT_HEIGHT: i32 = 24;
        const BUTTON_WIDTH: i32 = 100;
        const BUTTON_HEIGHT: i32 = 30;

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

        let y = y + h + SPACER;
        let h = INPUT_HEIGHT;

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

        let y = y + h + SPACER;
        let h = sz_wnd.cy - y - BORDER_MARGIN - SPACER - BUTTON_HEIGHT;

        unsafe {
            SetWindowPos(
                self.hwnd_listview,
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
        let w = BUTTON_WIDTH;
        let h = BUTTON_HEIGHT;

        unsafe {
            SetWindowPos(
                self.hwnd_button_cancel,
                None,
                sz_wnd.cx - w - BORDER_MARGIN,
                sz_wnd.cy - h - BORDER_MARGIN,
                w,
                h,
                SWP_FRAMECHANGED,
            )
        }
        .unwrap();

        unsafe {
            SetWindowPos(
                self.hwnd_button_ok,
                None,
                sz_wnd.cx - w - BORDER_MARGIN - BUTTON_WIDTH - SPACER,
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
        // if msg == WM_NOTIFY {
        //     println!("WM_NOTIFY code = {}", unsafe {
        //         (&*(lparam.0 as *const NMHDR)).code
        //     });
        // }

        match msg {
            WM_CREATE => {
                /* Create the window */
                // Retrieve the instance from the CreateWindowEx lpparam
                let create_params = unsafe { *(lparam.0 as *const CREATESTRUCTW) };

                let instance =
                    unsafe { Arc::from_raw(create_params.lpCreateParams as *const Instance) };

                // Create the window
                let dialog =
                    Arc::new(Self::create_window(instance, hwnd).expect("Failed to create window"));

                // Disable the parent
                unsafe {
                    let _ = EnableWindow(create_params.hwndParent, false);
                }

                // Initialize the list
                unsafe {
                    SendMessageW(
                        dialog.hwnd_listview,
                        LVM_SETEXTENDEDLISTVIEWSTYLE,
                        WPARAM(0),
                        LPARAM(LVS_EX_FULLROWSELECT as isize),
                    );
                }

                // Initialize columns
                let lvcol_base = LVCOLUMNW {
                    mask: LVCF_FMT | LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                    fmt: LVCFMT_LEFT,
                    ..Default::default()
                };

                const COLUMNS: [(PCWSTR, i32); 4] = [
                    (w!("Executable"), 192),
                    (w!("PID"), 64),
                    (w!("Arch"), 64),
                    (w!("Description"), 256),
                ];

                for (i, (str, cx)) in COLUMNS.iter().enumerate() {
                    let lvcol = LVCOLUMNW {
                        pszText: ::windows::core::PWSTR(str.0 as *mut u16),
                        cx: *cx,
                        ..lvcol_base
                    };

                    // TODO: Maybe move to util::listview
                    unsafe {
                        SendMessageW(
                            dialog.hwnd_listview,
                            LVM_INSERTCOLUMNW,
                            WPARAM(i),
                            LPARAM(std::ptr::addr_of!(lvcol) as isize),
                        );
                    }
                }

                let begin = std::time::Instant::now();

                let proc_infos = ProcessSnapshot::new()
                    .unwrap()
                    .map(|process| {
                        let proc_exe = process.process_name_buf();
                        let proc_id =
                            util::string_to_hstring(format!("{}", process.process_id())).unwrap();

                        let (proc_hicon, proc_desc, proc_name, proc_arch) = if let Ok(proc) =
                            Process::__tmp_open_ro(process.process_id())
                        {
                            let hicon = proc.icon().unwrap();
                            let arch = proc.arch().unwrap();

                            let (proc_name, proc_desc) = if let Ok(descs) = proc.file_descriptions()
                            {
                                let proc_name = descs
                                    .iter()
                                    .map(|desc| desc.get_string(FileInfoField::ProductName))
                                    .next()
                                    .transpose()
                                    .unwrap()
                                    .flatten();

                                let proc_desc = descs
                                    .iter()
                                    .map(|desc| desc.get_string(FileInfoField::FileDescription))
                                    .next()
                                    .transpose()
                                    .unwrap()
                                    .flatten();

                                (proc_name, proc_desc)
                            } else {
                                (None, None)
                            };

                            (hicon, proc_name, proc_desc, Some(arch))
                        } else {
                            println!("Failed to open {}", process.process_id());

                            (None, None, None, None)
                        };

                        (
                            proc_exe, proc_id, proc_hicon, proc_desc, proc_name, proc_arch,
                        )
                    })
                    .collect::<Vec<_>>();

                let dur = std::time::Instant::now() - begin;

                println!("Scanned applications in {:.2}s", dur.as_secs_f64());

                let icons = proc_infos
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (_, _, proc_hicon, _, _, _))| {
                        proc_hicon.as_ref().map(|proc_hicon| (i, proc_hicon))
                    })
                    .collect::<Vec<_>>();

                let himl_small = util::check(|| unsafe {
                    ImageList_Create(16, 16, ILC_COLOR32, icons.len() as i32, 0)
                })
                .unwrap();

                let icons = icons
                    .into_iter()
                    .map(|(i, &hicon)| unsafe {
                        let idx_image_list =
                            util::check(|| ImageList_ReplaceIcon(himl_small, -1, hicon)).unwrap();
                        assert_ne!(idx_image_list, -1, "Failed to add image to imagelist");
                        println!("Added image {} to index {idx_image_list}", hicon.0);

                        (i, (idx_image_list, hicon))
                    })
                    .collect::<HashMap<_, _>>();

                util::listview::set_image_list(dialog.hwnd_listview, himl_small, LVSIL_SMALL)
                    .unwrap();

                for (i, (proc_exe, proc_id, _, proc_desc, proc_name, proc_arch)) in
                    proc_infos.into_iter().enumerate()
                {
                    let idx_icon = icons.get(&i).map(|(i, _)| *i);
                    let lvitem = LVITEMW {
                        mask: LVIF_TEXT | LVIF_IMAGE,
                        iItem: i as i32,
                        iImage: idx_icon.unwrap_or(-1),
                        pszText: windows::core::PWSTR(
                            proc_name
                                .as_ref()
                                .map(|val| val.as_ptr() as *mut u16)
                                .unwrap_or(std::ptr::addr_of!(proc_exe) as *mut u16),
                        ),
                        ..Default::default()
                    };

                    // TODO: Maybe move to util::listview
                    unsafe {
                        SendMessageW(
                            dialog.hwnd_listview,
                            LVM_INSERTITEMW,
                            WPARAM(0),
                            LPARAM(std::ptr::addr_of!(lvitem) as isize),
                        );
                    }

                    for j in 1..COLUMNS.len() {
                        let item = LVITEMW {
                            iItem: i as i32,
                            iSubItem: j as i32,
                            ..Default::default()
                        };
                        let lvsubitem = match j {
                            1 => Some(LVITEMW {
                                mask: LVIF_TEXT,
                                pszText: windows::core::PWSTR(
                                    proc_id.as_wide().as_ptr() as *mut u16
                                ),
                                ..item
                            }),
                            2 => Some(LVITEMW {
                                mask: LVIF_TEXT,
                                pszText: windows::core::PWSTR(
                                    match proc_arch {
                                        Some(ProcessArchitecture::X64) => w!("x64"),
                                        Some(ProcessArchitecture::X86) => w!("x86"),
                                        None => w!(""),
                                    }
                                    .as_ptr() as *mut u16,
                                ),
                                ..item
                            }),
                            3 => Some(LVITEMW {
                                mask: LVIF_TEXT,
                                pszText: proc_desc
                                    .as_ref()
                                    .map(|val| windows::core::PWSTR(val.as_ptr() as *mut u16))
                                    .unwrap_or(item.pszText),
                                ..item
                            }),
                            _ => None,
                        };

                        if let Some(lvsubitem) = lvsubitem {
                            // TODO: Maybe move to util::listview
                            unsafe {
                                SendMessageW(
                                    dialog.hwnd_listview,
                                    LVM_SETITEMW,
                                    WPARAM(0),
                                    LPARAM(std::ptr::addr_of!(lvsubitem) as isize),
                                );
                            }
                        }
                    }
                }

                // Set the pointer
                unsafe {
                    // We need to make it a Arc<Dialog> to make it stay on the heap until WM_DESTROY
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::into_raw(dialog) as isize);
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
            WM_GETMINMAXINFO => {
                // For some reason the exact dimensions are not obeyed, so we need to add some extra
                const X_EXTRA: i32 = 20;
                const Y_EXTRA: i32 = 43;

                let min_max_info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };

                min_max_info.ptMinTrackSize.x = 250 + X_EXTRA;
                min_max_info.ptMinTrackSize.y = 200 + Y_EXTRA;
            }
            WM_NOTIFY => match unsafe { (*(lparam.0 as *const NMHDR)).code } {
                LVN_INSERTITEM => {
                    //println!("LVN_INSERTITEM");
                }
                LVN_GETDISPINFOW => {
                    // todo!("LVN_GETDISPINFOW should not be called")
                    // let disp_info = unsafe { &mut *(lparam.0 as *mut NMLVDISPINFOW) };

                    // FIXME: Please don't pretend like it is mut here, do it properly:
                    // disp_info.item.pszText =
                    //     ::windows::core::PWSTR(w!("Hello, world!").0 as *mut u16);
                }
                NM_CLICK => {
                    let info = unsafe { &mut *(lparam.0 as *mut NMITEMACTIVATE) };

                    let self_ = unsafe { &*Self::retrieve_self(hwnd).unwrap() };

                    unsafe {
                        let _ = EnableWindow(self_.hwnd_button_ok, info.iItem != -1);
                    }

                    println!("NM_CLICK item = {}", info.iItem);
                }
                //_ => (),
                _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            },
            WM_COMMAND => match util::loword(wparam.0 as u32) {
                IDC_DIALOG_OK => {
                    todo!("Item selected.");
                }
                IDC_DIALOG_CANCEL => unsafe { Self::destroy(hwnd) },
                _ => (),
            },
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }

        LRESULT(0)
    }

    fn retrieve_self(hwnd: HWND) -> Option<*const Dialog> {
        let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog };

        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }
}
