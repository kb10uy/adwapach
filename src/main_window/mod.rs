mod application;
mod view;

pub use self::application::MainWindowContext;
pub use self::view::MainWindow;

use std::sync::{Arc, Mutex};

use epi::backend::RepaintSignal;
use winit::{
    event_loop::{EventLoop, EventLoopProxy},
    window::WindowId,
};

/// User event type for `EventLoop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    RepaintRequested,
    NotifyIconMessage(WindowId, u16),
}

pub struct EventProxy(Mutex<EventLoopProxy<UserEvent>>);

impl EventProxy {
    /// Creates new proxy.
    pub fn new(event_loop: &EventLoop<UserEvent>) -> Arc<EventProxy> {
        Arc::new(EventProxy(Mutex::new(event_loop.create_proxy())))
    }
}

impl RepaintSignal for EventProxy {
    fn request_repaint(&self) {
        let locked = self.0.lock().expect("Event loop proxy was poisoned");
        locked
            .send_event(UserEvent::RepaintRequested)
            .expect("Cannot send repaint request");
    }
}
