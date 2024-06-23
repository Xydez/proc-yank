#![allow(dead_code)]

use std::sync::Arc;

use ffi::TranslationEntry;
use thiserror::Error;
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, BOOL, HANDLE},
        Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
        System::{
            Diagnostics::{
                Debug::{ReadProcessMemory, WriteProcessMemory},
                ToolHelp::{
                    CreateToolhelp32Snapshot, Module32NextW, Process32NextW, MODULEENTRY32W,
                    PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPPROCESS,
                },
            },
            Threading::{
                IsWow64Process, OpenProcess, QueryFullProcessImageNameW, PROCESS_ALL_ACCESS,
                PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
            },
        },
        UI::{Shell::ExtractIconExW, WindowsAndMessaging::HICON},
    },
};

use crate::util;

#[derive(Error, Debug)]
pub enum Error {
    #[error("An internal windows error occurred")]
    Windows(#[from] windows::core::Error),
    #[error("The OS returned an invalid string")]
    InvalidString,
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct ProcessSnapshot {
    handle: HANDLE,
}

impl ProcessSnapshot {
    /// Creates an iterable snapshot of the currently running processes
    pub fn new() -> Result<Self> {
        Ok(Self {
            handle: unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? },
        })
    }
}

impl Iterator for ProcessSnapshot {
    type Item = ProcessEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        unsafe { Process32NextW(self.handle, &mut entry) }.ok()?;

        Some(ProcessEntry { entry })
    }
}

impl Drop for ProcessSnapshot {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

pub struct ProcessEntry {
    entry: PROCESSENTRY32W,
}

impl ProcessEntry {
    /// Returns the process id of the process
    pub fn process_id(&self) -> u32 {
        self.entry.th32ProcessID
    }

    /// Returns the name of the process, or an error if it cannot be parsed into UTF-8
    pub fn process_name(&self) -> Result<String> {
        let str = self
            .entry
            .szExeFile
            .iter()
            .cloned()
            .take_while(|c| *c != 0)
            .collect::<Vec<u16>>();

        String::from_utf16(&str).map_err(|_| Error::InvalidString)

        //OsString::from_wide(&self.entry.szExeFile.iter().take_while(|c| c != 0).collect::<Vec<u16>>()).to_str().ok_or(Error::InvalidUtf8).map(str::to_string)
    }

    pub fn executable_path_full(&self) -> Vec<u16> {
        // let vec = Vec::with_capacity(1024);
        // unsafe {
        //     QueryFullProcessImageNameW(hproc, 0, vec.as_mut_ptr(), vec.capacity());
        // }

        todo!()
    }

    pub fn process_name_buf(&self) -> [u16; 260] {
        self.entry.szExeFile
    }

    pub fn process_descriptions(&self) -> Result<Vec<()>> {
        // See: https://stackoverflow.com/a/61711510

        todo!()
    }

    pub fn modules(&self) -> Result<ModuleSnapshot> {
        ModuleSnapshot::new(self.process_id())
    }

    pub fn open(&mut self) -> Result<Process> {
        Process::open(self.process_id())
    }
}

pub struct ModuleSnapshot {
    handle: HANDLE,
}

impl ModuleSnapshot {
    /// Creates an iterable snapshot of the currently running processes
    pub fn new(process_id: u32) -> Result<Self> {
        Ok(Self {
            handle: unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, process_id)? },
        })
    }
}

impl Iterator for ModuleSnapshot {
    type Item = Module;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };

        unsafe { Module32NextW(self.handle, &mut entry) }.ok()?;

        Some(Module { entry })
    }
}

impl Drop for ModuleSnapshot {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

pub struct Module {
    entry: MODULEENTRY32W,
}

impl Module {
    /// Returns the module id of the module
    pub fn module_id(&self) -> u32 {
        self.entry.th32ModuleID
    }

    /// Returns the name of the module, or an error if it cannot be parsed into UTF-8
    pub fn module_name(&self) -> Result<String> {
        let str = self
            .entry
            .szModule
            .iter()
            .cloned()
            .take_while(|c| *c != 0)
            .collect::<Vec<u16>>();

        String::from_utf16(&str).map_err(|_| Error::InvalidString)
    }

    pub fn module_addr(&self) -> *const std::ffi::c_void {
        self.entry.modBaseAddr as *const std::ffi::c_void
    }
}

pub struct Process {
    process_id: u32,
    process_handle: HANDLE,
}

impl Process {
    /// Opens the memory of the given process id
    pub fn open(process_id: u32) -> Result<Process> {
        let process_handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, process_id)? };

        Ok(Process {
            process_id,
            process_handle,
        })
    }

    pub fn __tmp_open_ro(process_id: u32) -> Result<Process> {
        let process_handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, false, process_id)? };

        Ok(Process {
            process_id,
            process_handle,
        })
    }

    /// Returns the process id opened
    pub fn process_id(&self) -> u32 {
        self.process_id
    }

    pub fn arch(&self) -> windows::core::Result<ProcessArchitecture> {
        let is_x64 = {
            let mut is_x64 = BOOL(0);
            util::check(|| unsafe {
                IsWow64Process(self.process_handle, std::ptr::addr_of_mut!(is_x64))
            })??;
            is_x64
        };

        let arch = match is_x64.as_bool() {
            true => ProcessArchitecture::X64,
            false => ProcessArchitecture::X86,
        };

        Ok(arch)
    }

    pub fn executable_path(&self) -> Result<windows::core::HSTRING> {
        let mut len: u32 = 1024;
        let mut buffer = vec![0; len as usize];

        unsafe {
            QueryFullProcessImageNameW(
                self.process_handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                std::ptr::addr_of_mut!(len),
            )?
        }

        Ok(windows::core::HSTRING::from_wide(&buffer[0..len as usize])?)
    }

    /// The caller is responsible for freeing the HICON after use
    /// Extracts the first 16x16 icon found in a file
    pub fn icon(&self) -> Result<Option<HICON>> {
        // let hicon = unsafe {
        //     ExtractIconW(
        //         HINSTANCE(GetModuleHandleW(None)?.0),
        //         &self.executable_path()?,
        //         0,
        //     )
        // };

        let mut hicon = HICON::default();
        unsafe {
            ExtractIconExW(
                &self.executable_path()?,
                0,
                None,
                Some(std::ptr::addr_of_mut!(hicon)),
                1,
            );
        }

        Ok(if hicon.is_invalid() {
            None
        } else {
            Some(hicon)
        })
    }

    pub fn file_descriptions(&self) -> Result<Vec<FileInfo>> {
        let executable_path = self.executable_path()?;

        let version_info_size = util::check(|| unsafe {
            GetFileVersionInfoSizeW(PCWSTR(executable_path.as_ptr()), None)
        })?;

        assert_ne!(version_info_size, 0, "Failed to get file version info size");

        let version_info_vec = {
            let mut vec: Vec<u8> = vec![0; version_info_size as usize];

            unsafe {
                GetFileVersionInfoW(
                    PCWSTR(executable_path.as_ptr()),
                    0,
                    version_info_size,
                    vec.as_mut_ptr() as *mut std::ffi::c_void,
                )?;
            }

            Arc::new(vec)
        };

        // 2. Get translation array from version info vec
        let translation_array: Vec<TranslationEntry> = {
            let mut translation_array = std::ptr::null_mut();
            let mut translation_array_size = 0;

            unsafe {
                VerQueryValueW(
                    version_info_vec.as_ptr() as *const std::ffi::c_void,
                    w!("\\VarFileInfo\\Translation"),
                    std::ptr::addr_of_mut!(translation_array),
                    std::ptr::addr_of_mut!(translation_array_size),
                )
                .ok()?;
            }

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    translation_array as *mut u8,
                    translation_array_size as usize,
                )
            };

            bytes
                .chunks(std::mem::size_of::<TranslationEntry>())
                .map(bytemuck::pod_read_unaligned::<TranslationEntry>)
                .collect()
        };

        // Get default system language
        // let default_lang = unsafe { GetSystemDefaultUILanguage() };

        let infos = translation_array
            .iter()
            .map(|entry| FileInfo {
                data: version_info_vec.clone(),
                language: entry.language,
                code_page: entry.code_page,
            })
            .collect();

        Ok(infos)
    }

    /// Reads the memory of the open process at an address into a buffer
    pub fn read(&self, address: *const std::ffi::c_void, buffer: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0;

        // Any process that has a handle with PROCESS_VM_READ access can call the function.
        unsafe {
            ReadProcessMemory(
                self.process_handle,
                address,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                buffer.len(),
                Some(&mut bytes_read),
            )
        }
        .map(|_| bytes_read)
        .map_err(|_| Error::Windows(windows::core::Error::from_win32()))
    }

    /// Reads the memory of the open process at an address and returns a buffer containing the data
    pub fn read_array<const N: usize>(&self, address: *const std::ffi::c_void) -> Result<[u8; N]> {
        let mut buf = [0; N];

        self.read(address, &mut buf)?;

        Ok(buf)
    }

    /// Writes the contents of a buffer to a given address within the memory of the open process and return the number of bytes written
    pub fn write(&self, address: *const std::ffi::c_void, buffer: &[u8]) -> Result<usize> {
        let mut bytes_written = 0;

        unsafe {
            WriteProcessMemory(
                self.process_handle,
                address,
                buffer.as_ptr() as *const std::ffi::c_void,
                buffer.len(),
                Some(&mut bytes_written),
            )
        }
        .map(|_| bytes_written)
        .map_err(|_| Error::Windows(windows::core::Error::from_win32()))
    }

    /* READ & WRITE EXTENSIONS */
    pub fn read_ptr(&self, address: *const std::ffi::c_void) -> Result<*const std::ffi::c_void> {
        self.read_u32(address)
            .map(|value| value as *const std::ffi::c_void)
    }

    pub fn read_u32(&self, address: *const std::ffi::c_void) -> Result<u32> {
        self.read_array(address).map(u32::from_ne_bytes)
    }

    pub fn read_i32(&self, address: *const std::ffi::c_void) -> Result<i32> {
        self.read_array(address).map(i32::from_ne_bytes)
    }

    pub fn read_f32(&self, address: *const std::ffi::c_void) -> Result<f32> {
        self.read_array(address).map(f32::from_ne_bytes)
    }

    pub fn read_u16(&self, address: *const std::ffi::c_void) -> Result<u16> {
        self.read_array(address).map(u16::from_ne_bytes)
    }

    pub fn read_u8(&self, address: *const std::ffi::c_void) -> Result<u8> {
        self.read_array::<1>(address).map(|arr| arr[0])
    }

    pub fn read_bool(&self, address: *const std::ffi::c_void) -> Result<bool> {
        Ok(self.read_u8(address)? != 0)
    }

    pub fn write_u32(&self, address: *const std::ffi::c_void, value: u32) -> Result<()> {
        self.write(address, &value.to_ne_bytes()).map(|_| ())
    }

    pub fn write_i32(&self, address: *const std::ffi::c_void, value: i32) -> Result<()> {
        self.write(address, &value.to_ne_bytes()).map(|_| ())
    }

    pub fn write_f32(&self, address: *const std::ffi::c_void, value: f32) -> Result<()> {
        self.write(address, &value.to_ne_bytes()).map(|_| ())
    }

    pub fn write_u16(&self, address: *const std::ffi::c_void, value: u16) -> Result<()> {
        self.write(address, &value.to_ne_bytes()).map(|_| ())
    }

    pub fn write_u8(&self, address: *const std::ffi::c_void, value: u8) -> Result<()> {
        self.write(address, &[value]).map(|_| ())
    }

    pub fn write_bool(&self, address: *const std::ffi::c_void, value: bool) -> Result<()> {
        self.write_u8(address, if value { 0x01 } else { 0x00 })
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.process_handle) };
    }
}

pub struct FileInfo {
    data: Arc<Vec<u8>>,
    language: u16,
    code_page: u16,
}

impl FileInfo {
    pub fn language(&self) -> u16 {
        self.language
    }

    pub fn code_page(&self) -> u16 {
        self.code_page
    }

    pub fn get_string(&self, field: FileInfoField) -> Result<Option<HSTRING>> {
        get_file_string(&self.data, self.language, self.code_page, field)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessArchitecture {
    X64,
    X86,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInfoField {
    Comments,
    InternalName,
    ProductName,
    CompanyName,
    LegalCopyright,
    ProductVersion,
    FileDescription,
    LegalTrademarks,
    PrivateBuild,
    FileVersion,
    OriginalFilename,
    SpecialBuild,
}

impl FileInfoField {
    pub fn field_name(&self) -> &'static str {
        match self {
            Self::Comments => "Comments",
            Self::InternalName => "InternalName",
            Self::ProductName => "ProductName",
            Self::CompanyName => "CompanyName",
            Self::LegalCopyright => "LegalCopyright",
            Self::ProductVersion => "ProductVersion",
            Self::FileDescription => "FileDescription",
            Self::LegalTrademarks => "LegalTrademarks",
            Self::PrivateBuild => "PrivateBuild",
            Self::FileVersion => "FileVersion",
            Self::OriginalFilename => "OriginalFilename",
            Self::SpecialBuild => "SpecialBuild",
        }
    }
}

mod ffi {
    #[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
    #[repr(C, packed)]
    pub struct TranslationEntry {
        pub language: u16,
        pub code_page: u16,
    }
}

fn get_file_string(
    version_info_vec: &[u8],
    language: u16,
    code_page: u16,
    field: FileInfoField,
) -> Result<Option<HSTRING>> {
    let file_description_key = util::string_to_hstring(format!(
        "\\StringFileInfo\\{language:04x}{code_page:04x}\\{}",
        field.field_name()
    ))?;

    let file_description: Vec<u16> = {
        let mut file_description = std::ptr::null_mut();
        let mut file_description_size = 0;

        let ret = unsafe {
            VerQueryValueW(
                version_info_vec.as_ptr() as *const std::ffi::c_void,
                &file_description_key,
                std::ptr::addr_of_mut!(file_description) as *mut *mut std::ffi::c_void,
                std::ptr::addr_of_mut!(file_description_size),
            )
        };
        if !ret.as_bool() {
            return Ok(None);
        }

        let bytes = unsafe {
            std::slice::from_raw_parts(
                file_description as *mut u8,
                std::mem::size_of::<u16>() * file_description_size as usize,
            )
        };

        bytes
            .chunks_exact(std::mem::size_of::<u16>())
            .map(|chunk| {
                u16::from_ne_bytes(chunk.try_into().expect("chunk size should always be 2"))
            })
            .collect()
    };

    let str = HSTRING::from_wide(&file_description)?;

    Ok(Some(str))
}
