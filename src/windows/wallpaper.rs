//! Provides desktop wallpaper manipulation.

use std::{ffi::OsString, os::windows::prelude::OsStringExt, slice::from_raw_parts};

use anyhow::{Context, Result};
use vek::Vec2;
use windows::{
    core::PCWSTR,
    Win32::{
        System::Com::{CoCreateInstance, CLSCTX_ALL},
        UI::Shell::{DesktopWallpaper, IDesktopWallpaper},
    },
};

/// Represents a monitor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Monitor {
    /// Monitor ID WSTR, which contains NUL word.
    id: Box<[u16]>,

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

            monitors.push(Monitor { id, position, size })
        }

        Ok(monitors)
    }
}
