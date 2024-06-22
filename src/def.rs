use windows::core::PCWSTR;

/* Constants */
/// See definition in CommCtrl.h
#[allow(dead_code)]
pub const LPSTR_TEXTCALLBACKW: PCWSTR = PCWSTR(usize::MAX as *mut u16);
