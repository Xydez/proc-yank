use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};

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
