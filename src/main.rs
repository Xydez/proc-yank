use windows::Win32::{
    Foundation::HMODULE,
    System::{
        Environment::GetCommandLineW,
        LibraryLoader::GetModuleHandleW,
        Threading::{GetStartupInfoW, STARTUPINFOW},
    },
};

mod dialog;
mod gui;
mod id;
mod util;
mod win_main;

fn main() {
    let h_instance = unsafe { GetModuleHandleW(None).unwrap() };
    let mut si = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    unsafe { GetStartupInfoW(&mut si) };
    let cmd_show = si.wShowWindow as i32;

    let command_line = unsafe { GetCommandLineW() };

    unsafe {
        win_main::win_main(
            h_instance,
            HMODULE::default(),
            command_line,
            cmd_show as u32,
        );
    }
}
