#![cfg_attr(production, windows_subsystem = "windows")]

mod application;
mod background;
mod egui;
mod mvvm;
mod windows;

use crate::{
    application::{Application, ApplicationView, ApplicationViewModel},
    background::load_monitor_info,
    egui::{EguiEvent, EguiWindow},
    windows::{initialize_com, terminate_com},
};

use std::sync::Arc;

use anyhow::Result;
use flexi_logger::Logger;
use log::error;
use tokio::runtime::{Builder, Runtime};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> Result<()> {
    Logger::try_with_env()?.start()?;
    let event_loop = EventLoop::with_user_event();
    let runtime = build_runtime()?;
    initialize_com(false)?;

    let application = Application::new();
    let application_viewmodel = ApplicationViewModel::new(application.clone());
    let application_view = ApplicationView::new(application_viewmodel)?;
    let mut application_window = runtime.block_on(EguiWindow::create(
        &event_loop,
        runtime.clone(),
        application_view,
    ))?;

    // Run async tasks
    runtime.spawn(load_monitor_info(application));

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

fn build_runtime() -> Result<Arc<Runtime>> {
    let tokio_runtime = Builder::new_multi_thread()
        .on_thread_start(|| match initialize_com(false) {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to initialize com for thread: {e}");
            }
        })
        .on_thread_stop(|| {
            terminate_com();
        })
        .build()?;

    Ok(Arc::new(tokio_runtime))
}
