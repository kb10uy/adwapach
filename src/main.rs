mod main_window;
mod windows;

use crate::{
    main_window::{MainWindow, UserEvent},
    windows::{initialize_com, Wallpaper},
};

use anyhow::Result;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

#[async_std::main]
async fn main() -> Result<()> {
    initialize_com()?;

    let wallpaper = Wallpaper::new()?;
    let monitors = wallpaper.monitors()?;
    println!("This system has {} monitor(s)", monitors.len());

    for (i, monitor) in monitors.iter().enumerate() {
        println!("Monitor #{i}");
        println!("ID    : {}", monitor.id_as_string());
        println!("Extent: {} / {}", monitor.position(), monitor.size());
        println!();
    }

    let event_loop = EventLoop::with_user_event();

    let mut main_window = MainWindow::create(&event_loop).await?;
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

    // Ok(())
}
