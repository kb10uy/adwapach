mod notify_icon;
mod wallpaper;

use std::ptr::null;

use anyhow::Result;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

pub use self::notify_icon::NotifyIcon;
pub use self::wallpaper::{Monitor, Wallpaper};

/// Initializes COM.
pub fn initialize_com() -> Result<()> {
    unsafe {
        CoInitializeEx(null(), COINIT_MULTITHREADED)?;
    }
    Ok(())
}
