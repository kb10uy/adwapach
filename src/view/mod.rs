pub mod application;
pub mod event;

pub use self::event::{EguiEvent, EventProxy};

use std::sync::Arc;

use epi::App;
use winit::window::{Icon, Window};

/// View struct should implement this trait.
pub trait View<E: EguiEvent>: App {
    /// Attaches window to this view.
    fn attach_window(&mut self, window: &Window, event_proxy: Arc<EventProxy<E>>);

    fn get_icon(&self) -> Option<Icon>;
}
