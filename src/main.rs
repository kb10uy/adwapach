#![cfg_attr(production, windows_subsystem = "windows")]

mod egui;
mod model;
mod view;
mod windows;

use crate::{
    egui::EguiWindow,
    view::{application::ApplicationView, RepaintableEvent},
    windows::{initialize_com, Wallpaper},
};

use std::{
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use model::application::Application;
use pollster::FutureExt as _;
use tokio::spawn;
use winit::{event::Event, event_loop::EventLoop};

fn main() -> Result<()> {
    let event_loop = EventLoop::with_user_event();

    let application_model = Application::new();
    let application_view = ApplicationView::new(application_model.clone());
    let mut application_window = EguiWindow::create(&event_loop, application_view).block_on()?;

    // Run business logic thread
    spawn(run_application_task(application_model));

    // Run UI thread
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event } => {
            if window_id == application_window.window_id() {
                application_window.update_with_event(event);
            }
        }
        Event::RedrawRequested(window_id) => {
            if window_id == application_window.window_id() {
                application_window.redraw();
            }
        }
        Event::MainEventsCleared => {
            application_window.redraw();
        }
        Event::UserEvent(ue) => {
            if ue.should_repaint() {
                *control_flow = application_window.redraw();
            }

            if let Some((window_id, visible)) = ue.should_change_window() {
                if window_id == application_window.window_id() {
                    application_window.set_visibility(visible);
                }
            }
        }
        _ => (),
    });
}

async fn run_application_task(main_app: Arc<Mutex<Application>>) -> Result<()> {
    initialize_com()?;

    let wallpaper = Wallpaper::new()?;
    let monitors = wallpaper.monitors()?;
    {
        let mut locked = main_app.lock().expect("Poisoned");
        locked.set_monitors(monitors);
    }

    loop {
        sleep(Duration::from_secs(1));
    }
}
