#[repr(C)]
struct LASTINPUTINFO {
    cbSize: u32,
    dwTime: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> BOOL;
    fn GetTickCount() -> u32;
}

type BOOL = i32;
type HWND = isize;

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> HWND;
    fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    fn GetWindowTextW(hwnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
    fn GetWindowThreadProcessId(hwnd: HWND, lpdwProcessId: *mut u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> BOOL;
}

#[link(name = "psapi")]
extern "system" {
    fn GetModuleBaseNameW(hProcess: isize, hModule: isize, lpBaseName: *mut u16, nSize: u32) -> u32;
}

const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const PROCESS_VM_READ: u32 = 0x0010;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug)]
pub enum MonitorError {
    WindowsApiError(i32),
    GetModuleBaseNameFailed,
}

impl std::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorError::WindowsApiError(code) => write!(f, "Windows API error: {}", code),
            MonitorError::GetModuleBaseNameFailed => write!(f, "GetModuleBaseName failed"),
        }
    }
}

impl std::error::Error for MonitorError {}

pub fn get_foreground_window_info() -> Result<WindowInfo, MonitorError> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == 0 {
        return Err(MonitorError::WindowsApiError(-1));
    }

    let window_title = get_window_title(hwnd)?;
    let process_name = get_process_name(hwnd)?;

    Ok(WindowInfo {
        process_name,
        window_title,
    })
}

fn get_window_title(hwnd: HWND) -> Result<String, MonitorError> {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; (len + 1) as usize];
    let result = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), len + 1) };
    if result == 0 {
        return Err(MonitorError::WindowsApiError(-1));
    }

    let len = result as usize;
    Ok(String::from_utf16_lossy(&buffer[..len]))
}

fn get_process_name(hwnd: HWND) -> Result<String, MonitorError> {
    let mut process_id = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };

    if process_id == 0 {
        return Err(MonitorError::WindowsApiError(-1));
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process_id) };
    if handle == 0 {
        return Err(MonitorError::WindowsApiError(-1));
    }

    let mut buffer = vec![0u16; 256];
    let result = unsafe { GetModuleBaseNameW(handle, 0, buffer.as_mut_ptr(), buffer.len() as u32) };

    unsafe { CloseHandle(handle) };

    if result == 0 {
        return Err(MonitorError::GetModuleBaseNameFailed);
    }

    let len = result as usize;
    Ok(String::from_utf16_lossy(&buffer[..len]))
}

pub fn get_idle_duration_ms() -> Result<u32, MonitorError> {
    let mut last_input_info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    let success = unsafe { GetLastInputInfo(&mut last_input_info) };
    if success == 0 {
        return Err(MonitorError::WindowsApiError(-1));
    }

    let current_time = unsafe { GetTickCount() };
    Ok(current_time.wrapping_sub(last_input_info.dwTime))
}

pub fn is_idle() -> Result<bool, MonitorError> {
    let idle_ms = get_idle_duration_ms()?;
    Ok(idle_ms >= 300000)
}