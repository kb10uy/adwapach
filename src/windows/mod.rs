mod notify_icon;
mod popup_menu;
mod wallpaper;

use std::ptr::null;

use anyhow::Result;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        COINIT_MULTITHREADED,
    },
    UI::Shell::DefSubclassProc,
};

pub use self::notify_icon::NotifyIcon;
pub use self::popup_menu::{MenuItem, PopupMenu};
pub use self::wallpaper::{Monitor, WallpaperInterface};

/// Initializes COM.
pub fn initialize_com(multi_threaded: bool) -> Result<()> {
    unsafe {
        if multi_threaded {
            CoInitializeEx(null(), COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE)?;
        } else {
            CoInitializeEx(null(), COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)?;
        }
    }
    Ok(())
}

/// Uninitializes COM.
pub fn terminate_com() {
    unsafe {
        CoUninitialize();
    }
}

/// Proxies subclass window procedure to Rust objects.
pub struct SubclassProxy(Box<dyn Fn(HWND, u32, WPARAM, LPARAM) -> bool + Send + Sync + 'static>);

impl SubclassProxy {
    /// Creates new proxy.
    pub fn new(
        f: impl Fn(HWND, u32, WPARAM, LPARAM) -> bool + Send + Sync + 'static,
    ) -> SubclassProxy {
        SubclassProxy(Box::new(f))
    }
}

/// Processes subclass message.
pub unsafe extern "system" fn subclass_window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    _data: usize,
) -> LRESULT {
    let proxy = &mut *(id as *mut SubclassProxy);
    let processed = (proxy.0)(hwnd, message, wparam, lparam);
    if processed {
        LRESULT(1)
    } else {
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}
