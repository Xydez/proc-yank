use std::sync::Arc;

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HMODULE},
        UI::{
            Controls::{
                InitCommonControlsEx, ICC_LISTVIEW_CLASSES, ICC_STANDARD_CLASSES,
                INITCOMMONCONTROLSEX,
            },
            WindowsAndMessaging::SW_SHOW,
        },
    },
};

use crate::window::{app::App, dialog::Dialog};

#[derive(Debug)]
pub struct Instance {
    pub h_instance: HINSTANCE,
    pub n_cmd_show: u32,
}

impl Instance {
    pub fn new(
        h_instance: HMODULE,
        _h_prev_instance: HMODULE,
        _p_cmd_line: PCWSTR,
        n_cmd_show: u32,
    ) -> Self {
        Instance {
            h_instance: h_instance.into(),
            n_cmd_show,
        }
    }
}

pub unsafe fn win_main(
    h_instance: HMODULE,
    h_prev_instance: HMODULE,
    p_cmd_line: PCWSTR,
    n_cmd_show: u32,
) {
    let instance = Arc::new(Instance::new(
        h_instance,
        h_prev_instance,
        p_cmd_line,
        n_cmd_show,
    ));

    // 1. Initialize common controls
    InitCommonControlsEx(&INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_LISTVIEW_CLASSES,
    })
    .unwrap();

    // 2. Register classes
    App::register(&instance);
    Dialog::register(&instance);

    // 3. Create windows
    let app = App::create(instance).unwrap();

    println!("app.weak_count() == {}", Arc::weak_count(&app));
    println!("app.strong_count() == {}", Arc::strong_count(&app));

    // 4. Run the application
    println!("Showing window");
    app.show(Some(SW_SHOW)).unwrap();
    app.run().unwrap();
}
