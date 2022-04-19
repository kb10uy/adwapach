use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use epi::backend::RepaintSignal;
use winit::{
    event_loop::{EventLoop, EventLoopProxy},
    window::WindowId,
};

/// Defines user event for `EventProxy`.
pub trait EguiEvent: Debug + Clone + Send + 'static {
    /// Creates repaint event.
    fn repaint() -> Self;

    /// Creates window showing event.
    fn show_window(window_id: WindowId) -> Self;

    /// Creates window hiding event.
    fn hide_window(window_id: WindowId) -> Self;

    fn exit() -> Self;

    /// Checks whether we should repaint window.
    fn should_repaint(&self) -> bool;

    /// Checks whether we should change some window visibility.
    fn should_change_window(&self) -> Option<(WindowId, bool)>;

    /// Checks whether we should exit the application.
    fn should_exit(&self) -> bool;
}

/// Proxies window event.
pub struct EventProxy<E: 'static>(Mutex<EventLoopProxy<E>>);

impl<E: EguiEvent> EventProxy<E> {
    /// Creates new proxy.
    pub fn new(event_loop: &EventLoop<E>) -> Arc<EventProxy<E>> {
        Arc::new(EventProxy(Mutex::new(event_loop.create_proxy())))
    }

    /// Requests to show specified window.
    pub fn request_show(&self, window_id: WindowId) {
        let locked = self.0.lock().expect("Event loop proxy was poisoned");
        locked
            .send_event(E::show_window(window_id))
            .expect("Cannot send repaint request");
    }

    /// Requests to hide specified window.
    pub fn request_hide(&self, window_id: WindowId) {
        let locked = self.0.lock().expect("Event loop proxy was poisoned");
        locked
            .send_event(E::hide_window(window_id))
            .expect("Cannot send repaint request");
    }

    pub fn exit(&self) {
        let locked = self.0.lock().expect("Event loop proxy was poisoned");
        locked
            .send_event(E::exit())
            .expect("Cannot send repaint request");
    }
}

impl<E: EguiEvent> RepaintSignal for EventProxy<E> {
    fn request_repaint(&self) {
        let locked = self.0.lock().expect("Event loop proxy was poisoned");
        locked
            .send_event(E::repaint())
            .expect("Cannot send repaint request");
    }
}
