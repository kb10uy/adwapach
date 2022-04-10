mod application;
mod view;
mod window;

pub use self::application::MainWindowModel;
pub use self::view::MainWindowView;
pub use self::window::MainWindow;

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use epi::backend::RepaintSignal;
use winit::{
    event_loop::{EventLoop, EventLoopProxy},
    window::WindowId,
};

/// User event type for `EventLoop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    ExitRequested,
    RepaintRequested,
    NotifyIconMessage(WindowId, u16, i16, i16),
    MenuItem(WindowId, u32),
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

/// Creates Windows icon bitmap from image file bytes.
pub fn create_icon_buffer(image_memory: &[u8]) -> Result<(Vec<u8>, Vec<u8>, u32, u32)> {
    let icon_image = image::load_from_memory(image_memory)?;
    let mut bgra_image =
        Vec::with_capacity((icon_image.width() * icon_image.height() * 4) as usize);
    let mut mask_image = Vec::with_capacity((icon_image.width() * icon_image.height()) as usize);
    for pixel in icon_image.as_rgba8().context("Not 8bit RGBA")?.pixels() {
        bgra_image.extend_from_slice(&[pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]]);
        mask_image.push(pixel.0[3].wrapping_sub(u8::MAX));
    }
    Ok((
        bgra_image,
        mask_image,
        icon_image.width(),
        icon_image.height(),
    ))
}
