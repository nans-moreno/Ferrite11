use std::{io, mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    UI::{
        Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
        WindowsAndMessaging::SW_SHOWNORMAL,
    },
};

pub fn open_target(target: &str) -> Result<(), String> {
    let wide: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let init = unsafe {
        CoInitializeEx(
            ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if init < 0 && init != RPC_E_CHANGED_MODE {
        return Err(format!("Initialisation Windows impossible ({init:#x})."));
    }
    let mut request = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpFile: wide.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };
    let result = if unsafe { ShellExecuteExW(&mut request) } == 0 {
        Err(io::Error::last_os_error().to_string())
    } else {
        Ok(())
    };
    if init >= 0 {
        unsafe { CoUninitialize() };
    }
    result
}
