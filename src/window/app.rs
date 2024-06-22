use std::sync::Arc;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{UpdateWindow, COLOR_WINDOW, HBRUSH},
        UI::WindowsAndMessaging::{
            AppendMenuW, CreateMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GetMessageW, GetWindowLongPtrW, LoadCursorW, LoadIconW,
            PostQuitMessage, RegisterClassExW, SetMenu, SetWindowLongPtrW, ShowWindow,
            TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
            IDI_APPLICATION, MF_POPUP, MF_STRING, MSG, SHOW_WINDOW_CMD, WINDOW_STYLE, WM_CLOSE,
            WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSEXW, WNDCLASS_STYLES, WS_EX_WINDOWEDGE,
            WS_OVERLAPPEDWINDOW, WS_THICKFRAME,
        },
    },
};

use crate::{
    id::{IDM_ATTACH, IDM_EXIT},
    util,
};

use super::{dialog::Dialog, win_main::Instance};

const WINDOW_CLASS: PCWSTR = w!("myWindowClass");

pub struct App {
    pub instance: Arc<Instance>,
    pub hwnd: HWND,
    dialog: Option<Arc<Dialog>>,
}

impl App {
    pub fn register(instance: &Instance) {
        println!("App::register");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
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

        println!("Registering window class");
        unsafe { util::catch(util::check_error_code(RegisterClassExW(&wc), 0)) };
    }

    /// Create the main application window.
    ///
    /// To show it, see [`App::show`].
    ///
    /// To run the application loop, see [`App::run`].
    pub fn create(instance: Arc<Instance>) -> ::windows::core::Result<Arc<App>> {
        // Create the window
        let hwnd = util::check_handle(unsafe {
            CreateWindowExW(
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
                // TODO: We might want to send settings and such through `Instance`
                Some(std::sync::Arc::into_raw(instance) as *const std::ffi::c_void),
            )
        })
        .unwrap();

        let app = unsafe {
            // Clone the Arc in GWLP_USERDATA safely
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const App;

            assert!(
                !ptr.is_null(),
                "App pointer should not be null between WM_CREATE and WM_DESTROY"
            );

            Arc::increment_strong_count(ptr);
            Arc::from_raw(ptr)
        };

        Ok(app)
    }

    /// Show the main application window.
    ///
    /// `cmd_show` should usually be `SW_SHOW`.
    pub fn show(&self, cmd_show: Option<SHOW_WINDOW_CMD>) -> ::windows::core::Result<()> {
        let cmd_show = cmd_show.unwrap_or(SHOW_WINDOW_CMD(self.instance.n_cmd_show as i32));

        unsafe {
            let _ = ShowWindow(self.hwnd, cmd_show);
            UpdateWindow(self.hwnd).ok()?;
        }

        Ok(())
    }

    /// Run the main loop for the program
    pub fn run(&self) -> ::windows::core::Result<()> {
        let mut msg = MSG::default();

        loop {
            unsafe {
                match GetMessageW(&mut msg, None, 0, 0).0 {
                    -1 => {
                        // An error occurred
                        return util::get_last_error();
                    }
                    0 => {
                        // `WM_QUIT` has been received
                        break;
                    }
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                };
            }
        }

        Ok(())
    }

    extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                let instance = unsafe {
                    Arc::from_raw(
                        (*(lparam.0 as *const CREATESTRUCTW)).lpCreateParams as *const Instance,
                    )
                };

                let app =
                    Arc::new(Self::create_window(instance, hwnd).expect("Failed to create window"));

                unsafe {
                    // We need to make it a Arc<App> to make it stay on the heap until WM_DESTROY
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::into_raw(app) as isize);
                }

                println!("app[hwnd]={}", hwnd.0);
            }
            WM_CLOSE => {
                // An application can prompt the user for confirmation, prior to destroying a window, by processing the WM_CLOSE message and calling the DestroyWindow function only if the user confirms the choice. [source](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close)
                unsafe { DestroyWindow(hwnd) }.unwrap();
            }
            WM_DESTROY => {
                println!("App WM_DESTROY");

                // Drop the arc stored in GWLP_USERDATA. This does not mean that other `Arc`s to the App are dropped.
                let app =
                    unsafe { Arc::from_raw(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const App) };

                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, std::ptr::null::<App>() as isize) };

                drop(app);

                unsafe { PostQuitMessage(0) };
            }
            WM_COMMAND => match util::loword(wparam.0 as u32) {
                IDM_ATTACH => {
                    // TODO: Remove mut here(?) use borrow cell thingy
                    let app = unsafe {
                        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;

                        assert!(
                            !ptr.is_null(),
                            "App pointer should not be null between WM_CREATE and WM_DESTROY"
                        );

                        &mut *ptr
                    };

                    let dialog = Dialog::create(app).unwrap();

                    app.dialog = Some(dialog);
                    // TODO: How do we set dialog to none when it closes???
                }
                IDM_EXIT => {
                    unsafe { DestroyWindow(hwnd).unwrap() }
                    // unsafe { PostQuitMessage(0) }
                }
                _ => unimplemented!(),
            },
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }

        LRESULT(0)
    }

    fn create_window(instance: Arc<Instance>, hwnd: HWND) -> ::windows::core::Result<App> {
        unsafe {
            let h_menu = CreateMenu()?;
            let h_file_menu = CreateMenu()?;

            AppendMenuW(
                h_file_menu,
                MF_STRING,
                IDM_ATTACH as usize,
                w!("Attach to program"),
            )?;
            AppendMenuW(h_file_menu, MF_STRING, IDM_EXIT as usize, w!("Exit"))?;
            AppendMenuW(h_menu, MF_POPUP, h_file_menu.0 as usize, w!("File"))?;

            SetMenu(hwnd, h_menu)?;
        }

        Ok(App {
            instance,
            hwnd,
            dialog: None,
        })
    }
}
