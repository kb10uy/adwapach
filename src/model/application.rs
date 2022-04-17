use crate::{
    model::{EventManager, Observable, Subscription, WeakSubscription},
    windows::Monitor,
};

use std::sync::{Arc, Mutex};

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
    pub fn new() -> Arc<Mutex<Application>> {
        Arc::new(Mutex::new(Application {
            subscribers: EventManager::new(),
            monitors: vec![],
            wallpapers: vec![],
        }))
    }
}

impl Observable for Application {
    type Message = ApplicationEvent;

    fn subscribe<S>(&self, subscription: S) -> Subscription<ApplicationEvent>
    where
        S: Fn(ApplicationEvent) + Send + 'static,
    {
        self.subscribers.subscribe(subscription)
    }

    fn notify(&self, message: ApplicationEvent) {
        self.subscribers.notify(message);
    }
}

/// Represents an event in `Application`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationEvent {
    MonitorsUpdated,
    WallpapersUpdated,
}
