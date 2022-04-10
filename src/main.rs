// #![windows_subsystem = "windows"]

mod main_window;
mod windows;

use crate::{
    main_window::{MainWindow, MainWindowApp, UserEvent},
    windows::{initialize_com, Wallpaper},
};

use std::{
    sync::{Arc, Mutex},
    thread::{sleep, spawn},
    time::Duration,
};

use anyhow::Result;
use pollster::FutureExt as _;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

fn main() -> Result<()> {
    initialize_com()?;

    let main_window_app = Arc::new(Mutex::new(MainWindowApp::default()));

    let main_app_control = main_window_app.clone();
    spawn(move || run_application_task(main_app_control));

    let event_loop = EventLoop::with_user_event();
    let mut main_window = MainWindow::create(&event_loop, main_window_app).block_on()?;
    let main_window_id = main_window.window_id();

    event_loop.run(move |event, _, control_flow| {
        let new_flow = match event {
            Event::WindowEvent { window_id, event } if window_id == main_window_id => {
                main_window.on_window_event(event)
            }
            Event::RedrawRequested(window_id) if window_id == main_window_id => {
                main_window.on_redraw()
            }
            Event::MainEventsCleared | Event::UserEvent(UserEvent::RepaintRequested) => {
                main_window.after_events();
                None
            }
            Event::UserEvent(UserEvent::MenuItem(window_id, mid))
                if window_id == main_window_id =>
            {
                main_window.on_menu_select(mid);
                None
            }
            Event::UserEvent(UserEvent::NotifyIconMessage(window_id, msg, x, y))
                if window_id == main_window_id =>
            {
                main_window.on_notify_icon(msg, x, y);
                None
            }
            Event::UserEvent(UserEvent::ExitRequested) => Some(ControlFlow::Exit),
            _ => None,
        };
        if let Some(flow) = new_flow {
            *control_flow = flow;
        }
    });
}

fn run_application_task(main_app: Arc<Mutex<MainWindowApp>>) -> Result<()> {
    initialize_com()?;

    let wallpaper = Wallpaper::new()?;
    let monitors = wallpaper.monitors()?;
    {
        println!("{monitors:?}");
        let mut locked = main_app.lock().expect("Poisoned");
        locked.set_monitors(monitors);
    }

    loop {
        sleep(Duration::from_secs(1));
    }
}
