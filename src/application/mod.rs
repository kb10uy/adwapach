mod model;
mod view;
mod viewmodel;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub use self::model::Application;
pub use self::view::ApplicationView;
pub use self::viewmodel::ApplicationViewModel;

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
