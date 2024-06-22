use std::{ffi::OsString, str::FromStr};

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{
            GetLastError, SetLastError, ERROR_INVALID_WINDOW_HANDLE, ERROR_SUCCESS, HANDLE, HWND,
            MAX_PATH, RECT, SIZE,
        },
        Graphics::Gdi::{
            CreateFontIndirectW, DeleteObject, GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ,
        },
        UI::WindowsAndMessaging::{
            GetClientRect, MessageBoxW, SystemParametersInfoW, MB_ICONEXCLAMATION, MB_OK,
            NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        },
    },
};

// Probably garbage code
#[derive(Debug)]
pub struct GdiHandle<T: windows::core::Param<HGDIOBJ> + Clone> {
    handle: T,
}

impl<T: windows::core::Param<HGDIOBJ> + Clone> GdiHandle<T> {
    #[allow(dead_code)]
    pub fn new(handle: T) -> GdiHandle<T> {
        GdiHandle { handle }
    }

    #[allow(dead_code)]
    pub fn inner(&self) -> &T {
        &self.handle
    }
}

impl<T: windows::core::Param<HGDIOBJ> + Clone> Drop for GdiHandle<T> {
    fn drop(&mut self) {
        unsafe { DeleteObject(self.handle.clone()).expect("handle should live") }
    }
}

pub fn get_last_error() -> ::windows::core::Result<()> {
    let error: windows::core::Error =
        windows::core::HRESULT::from_win32(unsafe { GetLastError().0 }).into();

    if error.code() == ERROR_SUCCESS.into() {
        Ok(())
    } else {
        Err(error)
    }
}

pub fn check_handle<T>(n: T) -> ::windows::core::Result<T>
where
    T: Into<HANDLE> + Copy,
{
    {
        let n: HANDLE = n.into();
        if n.is_invalid() {
            return Err(get_last_error()
                .err()
                .unwrap_or(windows::core::Error::from_hresult(
                    ERROR_INVALID_WINDOW_HANDLE.to_hresult(),
                )));
        }
    }

    Ok(n)
}

/// Catches errors and creates a message box before panicking
pub unsafe fn catch<T>(return_value: windows::core::Result<T>) -> T {
    match return_value {
        Ok(value) => value,
        Err(error) => {
            MessageBoxW(
                None,
                &string_to_hstring(format!("Error {}", error.code())).unwrap(),
                &string_to_hstring(format!("Error {}: {}", error.code(), error.message())).unwrap(), // w!("Error"),
                MB_ICONEXCLAMATION | MB_OK,
            );

            panic!("A Windows error occurred: {}", error);
        }
    }
}

pub fn string_to_hstring(string: impl AsRef<str>) -> windows::core::Result<HSTRING> {
    use std::os::windows::ffi::OsStrExt;

    HSTRING::from_wide(
        &OsString::from_str(string.as_ref())
            .expect("method is infallible")
            .encode_wide()
            .collect::<Vec<_>>(),
    )
}

pub fn check<F, R>(f: F) -> ::windows::core::Result<R>
where
    F: FnOnce() -> R,
{
    unsafe {
        SetLastError(ERROR_SUCCESS);
    }

    let ret = f();

    let error = unsafe { GetLastError() };

    if error == ERROR_SUCCESS {
        Ok(ret)
    } else {
        Err(::windows::core::Error::from_hresult(error.to_hresult()))
    }
}

pub unsafe fn check_error_code<T>(return_value: T, error: T) -> windows::core::Result<T>
where
    T: PartialEq,
{
    if return_value == error {
        return Err(get_last_error().unwrap_err());
    }

    Ok(return_value)
}

#[allow(dead_code)]
pub unsafe fn get_non_client_metrics() -> NONCLIENTMETRICSW {
    let mut ncmetrics = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };

    SystemParametersInfoW(
        SPI_GETNONCLIENTMETRICS,
        std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        Some((&mut ncmetrics) as *mut NONCLIENTMETRICSW as *mut std::ffi::c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
    .unwrap();

    ncmetrics
}

pub struct FontWrapper(HFONT);

impl AsRef<HFONT> for FontWrapper {
    fn as_ref(&self) -> &HFONT {
        &self.0
    }
}

impl Drop for FontWrapper {
    fn drop(&mut self) {
        unsafe { DeleteObject(self.0) }.unwrap();
    }
}

/// Get the default system font.
///
/// Safety: The caller is responsible for calling `DeleteObject(..)`.
#[allow(dead_code)]
pub fn get_default_font() -> HFONT {
    let ncmetrics = unsafe { get_non_client_metrics() };

    // FontWrapper(unsafe {  })

    unsafe { CreateFontIndirectW(&ncmetrics.lfMessageFont) }
}

#[allow(dead_code)]
pub fn get_instance_name() -> ::windows::core::Result<HSTRING> {
    use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};

    let h_instance = unsafe { GetModuleHandleW(None)? };

    let mut sz_file_name = Box::new([0u16; MAX_PATH as usize]);
    let length = unsafe { GetModuleFileNameW(h_instance, &mut *sz_file_name) };

    if length == 0 {
        get_last_error()?;
    }

    let str = unsafe { ::windows::core::PCWSTR::from_raw(sz_file_name.as_ptr()).to_hstring()? };

    println!("{}", str);

    Ok(str)
}

/// Retrieves the lower WORD (16 bits) of a DWORD (32 bits)
pub fn loword(l: u32) -> u16 {
    (l & 0xffff) as u16
}

/// Retrieves the higher WORD (16 bits) of a DWORD (32 bits)
pub fn hiword(l: u32) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

/// Get the size of a piece of text. This does not take neither wrap nor
/// newlines into accounnt, for that please see [get_text_size_wrap].
pub fn get_text_size(hdc: HDC, text: &[u16]) -> SIZE {
    let mut size = SIZE::default();

    unsafe { GetTextExtentPoint32W(hdc, text, &mut size) }.unwrap();

    size
}

/// Successively splits off the text by `delim` until the point where it does't
/// surpass `width_bound`, then it marks off that as one row finished and
/// continues until the entire text is split into rows.
///
/// Returns the rows and their respective size. This does not take newlines into
/// account, see [get_text_size_wrap].
fn wrap_text_by(hdc: HDC, width_bound: i32, text: &[u16], delim: u16) -> Vec<(Vec<u16>, SIZE)> {
    let mut rows = Vec::new();
    let mut working_text = text.to_vec();

    loop {
        for i in 0.. {
            let new_text = working_text
                .split(|c| *c == delim)
                .rev()
                .skip(i)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(&(delim));

            let new_size = get_text_size(hdc, &new_text);

            if width_bound == -1 || new_size.cx <= width_bound {
                rows.push((
                    working_text.drain(0..new_text.len()).collect::<Vec<_>>(),
                    new_size,
                ));
                break;
            }
        }

        if working_text.is_empty() {
            break;
        }
    }

    rows
}

/// Calculate the dimensions needed to fit a piece of text within bounds
///
/// To have an unlimited x or y axis, set it to -1.
///
/// If the function returns `None`, it means the text did not fit within the bounds.
pub fn get_text_size_wrap(hdc: HDC, bounds: SIZE, text: &[u16]) -> Option<SIZE> {
    let size = get_text_size(hdc, text);

    if bounds.cy != -1 && size.cy > bounds.cy {
        return None;
    }

    // Split the text into "blocks" where each block is marked by a newline.
    // Then calculate the blocks' respective rows and add them into a single
    // vec.
    let rows = text
        .to_vec()
        .split(|c| *c == '\n' as u16)
        .flat_map(|block| wrap_text_by(hdc, bounds.cx, block, ' ' as u16))
        .collect::<Vec<_>>();

    let size = SIZE {
        cx: rows.iter().map(|(_, size)| size.cx).max().unwrap(),
        cy: rows.iter().map(|(_, size)| size.cy).sum(),
    };

    if bounds.cy != -1 && size.cy > bounds.cy {
        None
    } else {
        Some(size)
    }
}

#[allow(dead_code)]
pub fn get_client_rect(hwnd: impl windows::core::Param<HWND>) -> windows::core::Result<RECT> {
    let mut rect = RECT::default();

    unsafe { GetClientRect(hwnd, &mut rect) }?;

    Ok(rect)
}

pub mod listview {
    use windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{
            Controls::{LVITEMW, LVM_GETNEXTITEM, LVM_INSERTITEMW, LVNI_FOCUSED},
            WindowsAndMessaging::SendMessageW,
        },
    };

    /// You cannot use LVM_INSERTITEM to insert subitems. The iSubItem member of the LVITEM structure must be zero. See LVM_SETITEM for information on setting subitems.
    /// Use the iItem member to specify the zero-based index at which the new item should be inserted.
    #[allow(dead_code)]
    pub unsafe fn insert_item(hwnd: HWND, item: &LVITEMW) -> ::windows::core::Result<()> {
        let ret = super::check(|| {
            SendMessageW(
                hwnd,
                LVM_INSERTITEMW,
                WPARAM(0),
                LPARAM(std::ptr::addr_of!(item) as isize),
            )
        })?;

        println!("util::listview::insert_item ret={}", ret.0);

        assert_ne!(ret.0, -1, "Failed to insert item");

        Ok(())
    }

    #[allow(dead_code)]
    pub unsafe fn get_focused_item(hwnd: HWND) -> ::windows::core::Result<isize> {
        let ret = super::check(|| {
            SendMessageW(
                hwnd,
                LVM_GETNEXTITEM,
                WPARAM(usize::MAX),
                LPARAM(LVNI_FOCUSED as isize),
            )
        })?;

        assert_ne!(ret.0, -1, "Failed to get item");

        Ok(ret.0)
    }
}
