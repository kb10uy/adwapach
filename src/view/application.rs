use crate::{
    model::application::Application,
    view::{EventProxy, RepaintableEvent, View},
    windows::{MenuItem, NotifyIcon, PopupMenu},
};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use egui::Context;
use epi::{App, Frame};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{WM_CONTEXTMENU, WM_LBUTTONUP},
};
use winit::{
    platform::windows::WindowExtWindows,
    window::{Icon, Window, WindowId},
};

const APPLICATION_TITLE: &str = "Adwapach";

const ICON_IMAGE_PNG: &[u8] = include_bytes!("../../resources/Adwapach.png");
const NOTIFY_ICON_MESSAGE_ID: u32 = 1;

const MENU_ID_SHOW: u32 = 0x1001;
const MENU_ID_EXIT: u32 = 0x1002;
const TASK_MENU_ITEMS: &[MenuItem] = &[
    MenuItem("Show Window", MENU_ID_SHOW),
    MenuItem("Exit", MENU_ID_EXIT),
];

/// User event type for `Application`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationViewEvent {
    RepaintRequested,
    ShowRequested(WindowId),
    HideRequested(WindowId),
}

impl RepaintableEvent for ApplicationViewEvent {
    fn repaint() -> ApplicationViewEvent {
        ApplicationViewEvent::RepaintRequested
    }

    fn show_window(window_id: winit::window::WindowId) -> Self {
        ApplicationViewEvent::ShowRequested(window_id)
    }

    fn hide_window(window_id: winit::window::WindowId) -> Self {
        ApplicationViewEvent::HideRequested(window_id)
    }

    fn should_repaint(&self) -> bool {
        *self == ApplicationViewEvent::RepaintRequested
    }

    fn should_change_window(&self) -> Option<(WindowId, bool)> {
        match self {
            Self::ShowRequested(window_id) => Some((*window_id, true)),
            Self::HideRequested(window_id) => Some((*window_id, false)),
            _ => None,
        }
    }
}

pub struct ApplicationView {
    model: Arc<Mutex<Application>>,
    notify_icon: Option<NotifyIcon>,
    should_exit: Arc<AtomicBool>,
}

impl ApplicationView {
    pub fn new(model: Arc<Mutex<Application>>) -> ApplicationView {
        ApplicationView {
            model,
            notify_icon: None,
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl View<ApplicationViewEvent> for ApplicationView {
    fn should_exit_application(&self) -> bool {
        self.should_exit.load(Ordering::SeqCst)
    }

    fn attach_window(
        &mut self,
        window: &Window,
        event_proxy: Arc<EventProxy<ApplicationViewEvent>>,
    ) {
        let window_id = window.id();
        let hwnd = HWND(window.hwnd() as _);
        let exit_flag = self.should_exit.clone();

        // Create popup menu
        let menu_event_proxy = event_proxy.clone();
        let task_menu = PopupMenu::new(hwnd, TASK_MENU_ITEMS, move |mid| match mid {
            MENU_ID_SHOW => {
                menu_event_proxy.request_show(window_id);
            }
            MENU_ID_EXIT => {
                exit_flag.store(true, Ordering::SeqCst);
            }
        })
        .expect("Failed to register popup menu");

        // Create notify icon
        let notify_event_proxy = event_proxy.clone();
        let notify_icon = NotifyIcon::new(
            hwnd,
            NOTIFY_ICON_MESSAGE_ID,
            APPLICATION_TITLE,
            ICON_IMAGE_PNG,
            move |message, (x, y)| match message as u32 {
                WM_LBUTTONUP => {
                    menu_event_proxy.request_show(window_id);
                }
                WM_CONTEXTMENU => {
                    task_menu.track_at(x as i32, y as i32);
                }
            },
        )
        .expect("Failed to register taskbar icon");

        self.notify_icon = Some(notify_icon);
    }

    fn get_icon(&self) -> Option<Icon> {
        let (icon_image, w, h) = {
            let image = image::load_from_memory(ICON_IMAGE_PNG).ok()?;
            let w = image.width();
            let h = image.height();
            let icon_image = image.to_rgba8().to_vec();
            (icon_image, w, h)
        };

        Icon::from_rgba(icon_image, w, h).ok()
    }
}

impl App for ApplicationView {
    fn name(&self) -> &str {
        APPLICATION_TITLE
    }

    fn update(&mut self, ctx: &Context, _frame: &Frame) {
        todo!()
    }
}
