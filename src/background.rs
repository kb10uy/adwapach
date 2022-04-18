use crate::{model::application::Application, windows::WallpaperInterface};

use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;

/// Fetches monitor information and sets them to application model.
pub async fn load_monitor_info(application: Arc<Mutex<Application>>) -> Result<()> {
    let monitors = {
        let wallpaper = WallpaperInterface::new()?;
        wallpaper.monitors()?
    };

    {
        let mut locked = application.lock();
        locked.set_monitors(monitors);
    }

    Ok(())
}
