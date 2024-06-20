use std::{pin::Pin, sync::Arc};

use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, TRUE, WPARAM},
        Graphics::Gdi::{CreateFontIndirectW, UpdateWindow, COLOR_WINDOW, HBRUSH, HFONT},
        UI::WindowsAndMessaging::{
            AppendMenuW, CreateMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, EnumChildWindows, GetMessageW, GetWindowLongPtrW, LoadCursorW,
            LoadIconW, PostQuitMessage, RegisterClassExW, SendMessageW, SetMenu, SetWindowLongPtrW,
            ShowWindow, TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
            IDI_APPLICATION, MF_POPUP, MF_STRING, MSG, SHOW_WINDOW_CMD, WINDOW_STYLE, WM_CLOSE,
            WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WM_SETFONT, WNDCLASSEXW,
            WNDCLASS_STYLES, WS_EX_WINDOWEDGE, WS_OVERLAPPEDWINDOW, WS_THICKFRAME,
        },
    },
};

use crate::{
    id::{FILE_MENU_EXIT, FILE_MENU_NEW},
    util,
    window::dialog::Dialog,
};

use super::win_main::Instance;

const WINDOW_CLASS: PCWSTR = w!("myWindowClass");

#[derive(Debug)]
pub struct App {
    pub instance: Arc<Instance>,
    pub hwnd: HWND,
    //_hf_default: util::GdiHandle<HFONT>,
}

impl Drop for App {
    fn drop(&mut self) {
        println!("App::drop");
    }
}

impl App {
    pub fn register(instance: &Instance) {
        println!("App::register");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(Self::global_window_proc),
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

        println!("Registering window class");
        unsafe { util::catch(util::check_error_code(RegisterClassExW(&wc), 0)) };
    }

    pub unsafe fn create(instance: Arc<Instance>) -> windows::core::Result<Arc<Self>> {
        println!("App::create");

        // TODO: Kanske använda Arc::new_cyclic(|| ...) och skicka en weak istället(?) sedan i WM_NCCREATE göra om den till en Arc MUHAHAHAHAHAHAH
        let app = Arc::new_cyclic(|weak| {
            let hwnd = util::check_handle(CreateWindowExW(
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
                instance.h_instance,
                Some(std::sync::Weak::into_raw(std::sync::Weak::clone(weak))
                    as *const std::ffi::c_void),
            ))
            .unwrap();

            App { instance, hwnd }
        });

        println!("App initialized.");

        // let ncmetrics = util::get_non_client_metrics();
        //let hf_default = util::GdiHandle::new(CreateFontIndirectW(&ncmetrics.lfMessageFont));

        // println!(
        //     "Default font: {}",
        //     HSTRING::from_wide(&ncmetrics.lfMessageFont.lfFaceName).unwrap()
        // );

        //let _ = EnumChildWindows(
        //    hwnd,
        //    Some(Self::enum_child_proc),
        //    LPARAM(hf_default.inner().0),
        //);

        //let app = Box::pin(App {
        //instance,
        //hwnd,
        ////_hf_default: hf_default,
        //});

        // unsafe {
        //     SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*app.as_ref() as *const App as isize);
        // }

        Ok(app)
    }

    pub unsafe fn show(&self, cmd_show: Option<SHOW_WINDOW_CMD>) {
        println!("App::show");

        // SW_SHOW
        let _ = ShowWindow(
            self.hwnd,
            cmd_show.unwrap_or(windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD(
                self.instance.n_cmd_show as i32,
            )),
        );

        let _ = UpdateWindow(self.hwnd);
    }

    pub unsafe fn run(&self) {
        println!("App::run");

        // Step 3: The Message Loop
        let mut msg = MSG::default();

        println!("Running window loop");
        while GetMessageW(&mut msg, None, 0, 0).ok().is_ok() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        println!("Exiting");
    }

    extern "system" fn global_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        println!("[msg={msg}] window_proc");

        // Sent prior to the WM_CREATE message when a window is first created.
        if msg == WM_NCCREATE {
            let pself =
                unsafe { (*(lparam.0 as *const CREATESTRUCTW)).lpCreateParams as *const App };

            println!("[msg={msg}] Setting app pointer to {pself:p}");
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, pself as isize);
            }

            //return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        }

        println!("[msg={msg}] Retrieving app pointer");
        // Should reasonably be valid
        let p_app = unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const App;
            println!("[msg={msg}] ptr = {ptr:p}");

            if ptr.is_null() {
                None
            } else {
                Some(std::sync::Weak::from_raw(ptr))
            }
        };

        if let Some(app) = p_app.and_then(|p_app| p_app.upgrade()) {
            let result = app.window_proc(msg, wparam, lparam);

            if msg == WM_NCDESTROY {
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, std::ptr::null() as isize) };
                drop(app);
            } else {
                // Keep the weak count alive
                let _ = std::sync::Weak::into_raw(std::sync::Arc::downgrade(&app));
            }

            return result;
        } else {
            match msg {
                WM_CREATE => Self::add_menus(hwnd),
            }

            return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        }

        match msg {
            WM_CREATE => {
                //Self::add_menus(hwnd).unwrap();
            }
            WM_CLOSE => {
                println!("Destroying window");
                unsafe { DestroyWindow(hwnd) }.unwrap();
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                unsafe { PostQuitMessage(0) };
            }
            // TODO: Was it something about the lower half of wparam?
            WM_COMMAND => match wparam.0 as u32 {
                FILE_MENU_NEW => {
                    println!("display_dialog");

                    println!("COMMENTED OUT LMAO");
                    // Dialog::create(pself).unwrap();
                }
                FILE_MENU_EXIT => {
                    unsafe { PostQuitMessage(0) };
                }
                _ => (),
            },
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }

        // The WM_NCDESTROY message is sent after the child windows have been destroyed. In contrast, WM_DESTROY is sent before the child windows are destroyed.
        if msg == WM_NCDESTROY {
            // Release the arc
            println!("WM_NCDESTROY - dropping p_app");
            drop(p_app);
        } else {
            // Saving p_app
            let _ = std::sync::Weak::into_raw(p_app);
        }

        LRESULT(0)
    }

    fn window_proc(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {}

    unsafe extern "system" fn enum_child_proc(h_wnd: HWND, lparam: LPARAM) -> BOOL {
        let hf_default: HFONT = HFONT(lparam.0);

        SendMessageW(
            h_wnd,
            WM_SETFONT,
            WPARAM(hf_default.0 as usize),
            LPARAM(TRUE.0 as isize),
        );

        TRUE
    }

    fn add_menus(hwnd: HWND) -> ::windows::core::Result<()> {
        unsafe {
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
        }

        Ok(())
    }
}
