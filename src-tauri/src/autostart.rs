use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_SZ,
};

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "ScreenManager";

pub fn is_autostart_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY_CURRENT_USER;
        let path: Vec<u16> = RUN_KEY_PATH.encode_utf16().chain(std::iter::once(0)).collect();

        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), 0, KEY_ALL_ACCESS, &mut hkey).is_err() {
            return false;
        }

        let value_name: Vec<u16> = APP_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data: [u16; 512] = [0; 512];
        let mut data_size = (data.len() * std::mem::size_of::<u16>()) as u32;

        let result = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(data.as_mut_ptr() as *mut u8),
            Some(&mut data_size),
        );

        let _ = RegCloseKey(hkey);

        result.is_ok()
    }
}

pub fn enable_autostart() -> bool {
    let exe_path = match get_exe_path() {
        Some(path) => path,
        None => return false,
    };

    unsafe {
        let mut hkey = HKEY_CURRENT_USER;
        let path: Vec<u16> = RUN_KEY_PATH.encode_utf16().chain(std::iter::once(0)).collect();

        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            0,
            None,
            windows::Win32::System::Registry::REG_OPEN_CREATE_OPTIONS(0),
            KEY_ALL_ACCESS,
            None,
            &mut hkey,
            None,
        ).is_err() {
            return false;
        }

        let value_name: Vec<u16> = APP_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let value_data: Vec<u8> = format!("\"{}\"", exe_path.display())
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect();

        let result = RegSetValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            0,
            REG_SZ,
            Some(&value_data),
        );

        let _ = RegCloseKey(hkey);

        result.is_ok()
    }
}

pub fn disable_autostart() -> bool {
    unsafe {
        let mut hkey = HKEY_CURRENT_USER;
        let path: Vec<u16> = RUN_KEY_PATH.encode_utf16().chain(std::iter::once(0)).collect();

        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), 0, KEY_ALL_ACCESS, &mut hkey).is_err() {
            return false;
        }

        let value_name: Vec<u16> = APP_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let result = RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()));

        let _ = RegCloseKey(hkey);

        if result.is_ok() {
            return true;
        }

        if let Err(e) = result {
            return e.code() == WIN32_ERROR(ERROR_FILE_NOT_FOUND.0).into();
        }

        false
    }
}

fn get_exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}