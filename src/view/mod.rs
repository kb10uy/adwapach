pub mod application;
pub mod event;

pub use self::event::{EventProxy, RepaintableEvent};

use std::sync::Arc;

use epi::App;
use winit::window::{Icon, Window};

/// View struct should implement this trait.
pub trait View<E: RepaintableEvent>: App {
    /// Should exit the whole application?
    fn should_exit_application(&self) -> bool;

    /// Attaches window to this view.
    fn attach_window(&mut self, window: &Window, event_proxy: Arc<EventProxy<E>>);

    fn get_icon(&self) -> Option<Icon>;
}
