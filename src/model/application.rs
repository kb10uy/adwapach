use crate::{
    model::{EventManager, Observable, Subscription},
    windows::{Monitor, WallpaperInterface},
};

use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the positioning of wallpaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fitting {
    /// Not scaled, placed in center.
    Center,

    /// Not scaled, tiled.
    Tile,

    /// Scaled and filling the monitor, aspect ratio may change.
    Stretch,

    /// Scaled to be contained the whole image.
    Contain,

    /// Scaled to be covered the whole desktop.
    Cover,
}

/// Represents an item of wallpaper.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Wallpaper {
    uuid: Uuid,
    filename: String,
    fitting: Fitting,
}

impl Wallpaper {
    /// Constructs new instance with generated UUID.
    pub fn new(filename: impl Into<String>, fitting: Fitting) -> Wallpaper {
        Wallpaper {
            uuid: Uuid::new_v4(),
            filename: filename.into(),
            fitting,
        }
    }

    /// Gets assigned UUID.
    pub fn id(&self) -> Uuid {
        self.uuid
    }

    /// Gets filename.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Gets fitting strategy.
    pub fn fitting(&self) -> Fitting {
        self.fitting
    }

    /// Sets new fitting strategy.
    pub fn set_fitting(&mut self, fitting: Fitting) {
        self.fitting = fitting;
    }
}

/// Application model object.
pub struct Application {
    subscribers: EventManager<ApplicationEvent>,
    monitors: Vec<Monitor>,
    wallpapers: Vec<Wallpaper>,
}

impl Application {
    /// Constructs new model.
    pub fn new() -> Arc<Mutex<Application>> {
        Arc::new(Mutex::new(Application {
            subscribers: EventManager::new(),
            monitors: vec![],
            wallpapers: vec![],
        }))
    }

    /// Refers monitors.
    pub fn monitors(&self) -> &[Monitor] {
        &self.monitors
    }

    /// Refers wallpapers.
    pub fn wallpapers(&self) -> &[Wallpaper] {
        &self.wallpapers
    }

    /// Sets monitors information.
    pub fn set_monitors(&mut self, monitors: Vec<Monitor>) {
        self.monitors = monitors;
        self.subscribers.notify(ApplicationEvent::MonitorsUpdated);
    }

    /// Pushes new wallpaper.
    pub fn add_wallpaper(&mut self, wallpaper: Wallpaper) {
        self.wallpapers.push(wallpaper);
        self.subscribers.notify(ApplicationEvent::WallpapersUpdated);
    }

    /// Performs an operation for specified indexed item.
    pub fn update_wallpaper(&mut self, index: usize, op: WallpaperListOperation) {
        match op {
            WallpaperListOperation::Remove => {
                self.wallpapers.remove(index);
            }
            WallpaperListOperation::MoveUp if index > 0 => {
                self.wallpapers.swap(index, index - 1);
            }
            WallpaperListOperation::MoveDown if index + 1 < self.wallpapers.len() => {
                self.wallpapers.swap(index, index + 1);
            }
            WallpaperListOperation::SetFitting(f) => {
                self.wallpapers[index].set_fitting(f);
            }
            _ => (),
        }
        self.subscribers.notify(ApplicationEvent::WallpapersUpdated);
    }

    /// Applies selected wallpaper for selected monitor.
    pub fn apply_wallpaper_for_monitor(
        &self,
        monitor_index: usize,
        wallpaper_index: usize,
    ) -> Result<()> {
        let wpi = WallpaperInterface::new()?;
        wpi.set_wallpaper(
            self.monitors[monitor_index].id(),
            &self.wallpapers[wallpaper_index].filename,
        )?;
        Ok(())
    }
}

impl Observable for Application {
    type Message = ApplicationEvent;

    fn subscribe<S>(&mut self, subscription: S) -> Subscription<ApplicationEvent>
    where
        S: Fn(ApplicationEvent) + Send + Sync + 'static,
    {
        self.subscribers.subscribe(subscription)
    }

    fn notify(&mut self, message: ApplicationEvent) {
        self.subscribers.notify(message);
    }
}

/// Represents an action for wallpapers list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperListOperation {
    /// Removes this item.
    Remove,

    /// Moves it up.
    MoveUp,

    /// Moves it down.
    MoveDown,

    /// Sets new `Fitting` for this.
    SetFitting(Fitting),
}

/// Represents an event in `Application`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationEvent {
    MonitorsUpdated,
    WallpapersUpdated,
}
