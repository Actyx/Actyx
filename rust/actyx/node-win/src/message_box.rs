use std::iter::once;
use std::ptr::null_mut;
use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK, MB_SYSTEMMODAL};

pub fn create(title: &str, content: &str) -> anyhow::Result<()> {
    let lp_text: Vec<u16> = content.encode_utf16().chain(once(0)).collect();
    let lp_caption: Vec<u16> = title.encode_utf16().chain(once(0)).collect();

    let window_type = MB_OK | MB_ICONERROR | MB_SYSTEMMODAL;

    unsafe {
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-messageboxw#return-value
        // If the return value is zero, creating message box has failed
        match MessageBoxW(null_mut(), lp_text.as_ptr(), lp_caption.as_ptr(), window_type) {
            0 => anyhow::bail!("Error creating message box"),
            _ => Ok(()),
        }
    }
}
