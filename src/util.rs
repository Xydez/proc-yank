use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{ERROR_SUCCESS, HANDLE, MAX_PATH, SIZE},
        Graphics::Gdi::{CreateFontIndirectW, DeleteObject, GetTextExtentPoint32W, HDC, HFONT},
        UI::WindowsAndMessaging::{
            SystemParametersInfoW, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        },
    },
};

pub unsafe fn get_last_error() -> ::windows::core::Result<()> {
    let error = ::windows::core::Error::from_win32();

    if error.code() == ERROR_SUCCESS.into() {
        Ok(())
    } else {
        Err(error)
    }
}

pub unsafe fn check_handle<T>(n: T) -> ::windows::core::Result<T>
where
    T: Into<HANDLE> + Copy,
{
    {
        let n: HANDLE = n.into();
        if n.is_invalid() {
            get_last_error()?;
        }
    }

    Ok(n)
}

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

pub fn get_default_font() -> FontWrapper {
    let ncmetrics = unsafe { get_non_client_metrics() };

    FontWrapper(unsafe { CreateFontIndirectW(&ncmetrics.lfMessageFont) })
}

pub fn get_instance_name() -> ::windows::core::Result<HSTRING> {
    use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};

    let h_instance = unsafe { GetModuleHandleW(None)? };

    let mut sz_file_name = Box::new([0u16; MAX_PATH as usize]);
    let length = unsafe { GetModuleFileNameW(h_instance, &mut *sz_file_name) };

    if length == 0 {
        unsafe { get_last_error() }?;
    }

    let str = unsafe { ::windows::core::PCWSTR::from_raw(sz_file_name.as_ptr()).to_hstring()? };

    println!("{}", str);

    Ok(str)
}

pub fn loword(l: u32) -> u16 {
    (l & 0xffff) as u16
}

pub fn hiword(l: u32) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

/// Get the size of a piece of text. This does not take neither wrap nor
/// newlines into accounnt, for that please see [get_text_size_wrap].
pub unsafe fn get_text_size(hdc: HDC, text: &[u16]) -> SIZE {
    let mut size = SIZE::default();

    GetTextExtentPoint32W(hdc, text, &mut size).unwrap();

    size
}

/// Successively splits off the text by `delim` until the point where it does't
/// surpass `width_bound`, then it marks off that as one row finished and
/// continues until the entire text is split into rows.
///
/// Returns the rows and their respective size. This does not take newlines into
/// account, see [get_text_size_wrap].
unsafe fn wrap_text_by(
    hdc: HDC,
    width_bound: i32,
    text: &[u16],
    delim: u16,
) -> Vec<(Vec<u16>, SIZE)> {
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
pub unsafe fn get_text_size_wrap(hdc: HDC, bounds: SIZE, text: &[u16]) -> Option<SIZE> {
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
        .map(|block| wrap_text_by(hdc, bounds.cx, block, ' ' as u16))
        .flatten()
        .collect::<Vec<_>>();

    let size = SIZE {
        cx: rows.iter().map(|(_, size)| size.cx).max().unwrap(),
        cy: rows.iter().map(|(_, size)| size.cy).sum(),
    };

    return if bounds.cy != -1 && size.cy > bounds.cy {
        None
    } else {
        Some(size)
    };
}
