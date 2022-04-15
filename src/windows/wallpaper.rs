//! Provides desktop wallpaper manipulation.

use std::{
    collections::HashMap, ffi::OsString, mem::size_of, os::windows::prelude::OsStringExt,
    ptr::null, slice::from_raw_parts,
};

use anyhow::{Context, Result};
use vek::Vec2;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::BOOL,
        Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW},
        System::Com::{CoCreateInstance, CLSCTX_ALL},
        UI::{
            Shell::{DesktopWallpaper, IDesktopWallpaper},
            WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME,
        },
    },
};

/// Represents a monitor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Monitor {
    /// Monitor ID WSTR, which contains NUL word.
    id: Box<[u16]>,

    /// Monitor name.
    name: String,

    /// Top-left monitor position.
    position: Vec2<i32>,

    /// Physical size of this monitor.
    size: Vec2<i32>,
}

impl Monitor {
    /// Gets monitor ID as `String`.
    pub fn id_as_string(&self) -> String {
        OsString::from_wide(&self.id).to_string_lossy().to_string()
    }

    /// Gets monitor name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets monitor position.
    pub fn position(&self) -> Vec2<i32> {
        self.position
    }

    /// Gets monitor size.
    pub fn size(&self) -> Vec2<i32> {
        self.size
    }
}

/// Provides wallpaper manipulations.
#[derive(Debug)]
pub struct Wallpaper {
    interface: IDesktopWallpaper,
}

impl Wallpaper {
    /// Initializes `IDesktopWallpaper` internally.
    pub fn new() -> Result<Wallpaper> {
        let interface: IDesktopWallpaper = unsafe {
            CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
                .context("Failed to initialize IDesktopWallper")?
        };

        Ok(Wallpaper { interface })
    }

    /// Fetches connected monitors information.
    pub fn monitors(&self) -> Result<Vec<Monitor>> {
        let monitor_names = self.list_monitor_names()?;

        let monitor_count = unsafe { self.interface.GetMonitorDevicePathCount()? } as usize;

        let mut monitors = Vec::with_capacity(monitor_count);
        for i in 0..monitor_count {
            let id = unsafe {
                let monitor_id_ptr = self.interface.GetMonitorDevicePathAt(i as u32)?.0;
                let monitor_id_length = (0..std::isize::MAX)
                    .position(|i| *monitor_id_ptr.offset(i) == 0)
                    .context("Unterminated text")?;

                // Contain NUL word
                from_raw_parts(monitor_id_ptr, monitor_id_length + 1)
                    .to_vec()
                    .into_boxed_slice()
            };

            let rect = unsafe { self.interface.GetMonitorRECT(PCWSTR(id.as_ptr()))? };
            let position = Vec2::new(rect.left, rect.top);
            let size = Vec2::new(rect.right - rect.left, rect.bottom - rect.top);
            let name = monitor_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| format!("Monitor #{i}"));

            monitors.push(Monitor {
                id,
                name,
                position,
                size,
            })
        }

        Ok(monitors)
    }

    fn list_monitor_names(&self) -> Result<HashMap<Box<[u16]>, String>> {
        let mut display_device = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        let mut name_pairs = HashMap::new();
        let mut index = 0;
        loop {
            let device_name = unsafe {
                let hr = EnumDisplayDevicesW(PCWSTR(null()), index, &mut display_device, 0);
                if hr == BOOL(0) {
                    break;
                }

                display_device.DeviceName
            };

            unsafe {
                let hr = EnumDisplayDevicesW(
                    PCWSTR(device_name[..].as_ptr()),
                    0,
                    &mut display_device,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                );
                if hr == BOOL(0) {
                    break;
                }
            }

            let id_length = display_device
                .DeviceID
                .iter()
                .position(|&x| x == 0)
                .expect("Unterminated text");
            let id: Box<[u16]> = display_device.DeviceID[..(id_length + 1)].into();

            let string_length = display_device
                .DeviceString
                .iter()
                .position(|&x| x == 0)
                .expect("Unterminated text");
            let name: String = OsString::from_wide(&display_device.DeviceString[..string_length])
                .to_string_lossy()
                .to_string();

            name_pairs.insert(id, name);
            index += 1;
        }

        Ok(name_pairs)
    }
}
