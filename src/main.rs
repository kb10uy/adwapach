#![cfg_attr(production, windows_subsystem = "windows")]

mod egui;
mod model;
mod view;
mod windows;

use crate::{
    egui::EguiWindow,
    model::application::Application,
    view::{application::ApplicationView, EguiEvent},
    windows::{initialize_com, Wallpaper},
};

use std::{sync::Arc, thread::sleep, time::Duration};

use anyhow::Result;
use flexi_logger::Logger;
use log::error;
use parking_lot::Mutex;
use pollster::FutureExt as _;
use tokio::{runtime::Builder, spawn};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

struct AsyncApplicationTask {
    application: Arc<Mutex<Application>>,
}

fn main() -> Result<()> {
    Logger::try_with_env()?.start()?;
    let tokio_runtime = Builder::new_multi_thread().worker_threads(4).build()?;

    let event_loop = EventLoop::with_user_event();

    let application = Application::new();
    let application_view = ApplicationView::new(application.clone())?;
    let mut application_window = EguiWindow::create(&event_loop, application_view).block_on()?;

    // Run async tasks
    let task = AsyncApplicationTask { application };
    tokio_runtime.spawn(async_main(task));

    // Run UI thread
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event } => {
            if window_id == application_window.window_id() {
                application_window.update_with_event(event);
            }
        }
        Event::RedrawRequested(window_id) => {
            if window_id == application_window.window_id() {
                match application_window.redraw() {
                    Ok(f) => {
                        *control_flow = f;
                    }
                    Err(e) => {
                        error!("Redraw error: {}", e);
                    }
                }
            }
        }
        Event::MainEventsCleared => {
            application_window.on_event_cleared();
        }
        Event::UserEvent(ue) => {
            if ue.should_exit() {
                *control_flow = ControlFlow::Exit;
            }
            if ue.should_repaint() {
                application_window.on_event_cleared();
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

async fn async_main(task: AsyncApplicationTask) -> Result<()> {
    spawn(run_application_task(task.application));

    Ok(())
}

async fn run_application_task(application: Arc<Mutex<Application>>) -> Result<()> {
    initialize_com()?;

    let monitors = {
        let wallpaper = Wallpaper::new()?;
        wallpaper.monitors()?
    };

    {
        let mut locked = application.lock();
        locked.set_monitors(monitors);
    }

    loop {
        sleep(Duration::from_secs(1));
    }
}
