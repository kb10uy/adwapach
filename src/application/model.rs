use crate::{
    application::{Fitting, Wallpaper},
    mvvm::{EventManager, Observable, Subscription},
    windows::{Monitor, WallpaperInterface},
};

use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;

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

/// Represents an event in `Application`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationEvent {
    MonitorsUpdated,
    WallpapersUpdated,
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
