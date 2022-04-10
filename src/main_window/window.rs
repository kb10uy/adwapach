use std::sync::{Arc, Mutex};

use crate::{
    main_window::{EventProxy, MainWindowApp, UserEvent},
    windows::{MenuItem, NotifyIcon, PopupMenu},
};

use anyhow::{Context, Result};
use egui::{ClippedMesh, Context as EguiContext, RawInput, TexturesDelta};
use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit::State as EguiState;
use epi::App;
use log::error;
use wgpu::TextureView;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{WM_CONTEXTMENU, WM_LBUTTONUP},
};
use winit::{
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Icon, Window, WindowBuilder, WindowId},
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

const ENCODER_DESCRIPTION: wgpu::CommandEncoderDescriptor = wgpu::CommandEncoderDescriptor {
    label: Some("Egui Encoder"),
};

pub struct MainWindow {
    window: Window,
    event_proxy: Arc<EventProxy>,
    task_menu: PopupMenu,
    _notify_icon: NotifyIcon,
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    egui_context: EguiContext,
    egui_state: EguiState,
    egui_render_pass: EguiRenderPass,
    application: Arc<Mutex<MainWindowApp>>,
}

impl MainWindow {
    pub async fn create(
        event_loop: &EventLoop<UserEvent>,
        application: Arc<Mutex<MainWindowApp>>,
    ) -> Result<MainWindow> {
        let event_proxy = EventProxy::new(event_loop);

        // Create window
        let (icon_image, w, h) = {
            let image = image::load_from_memory(ICON_IMAGE_PNG)?;
            let w = image.width();
            let h = image.height();
            let icon_image = image.to_rgba8().to_vec();
            (icon_image, w, h)
        };
        let icon = Icon::from_rgba(icon_image, w, h)?;
        let window = WindowBuilder::new()
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_drag_and_drop(false)
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_window_icon(Some(icon))
            .with_title(APPLICATION_TITLE)
            .build(event_loop)?;
        let hwnd = HWND(window.hwnd() as _);
        let window_id = window.id();

        // Create popup menu
        let menu_event_proxy = event_proxy.clone();
        let task_menu = PopupMenu::new(hwnd, TASK_MENU_ITEMS, move |mid| {
            let locked = menu_event_proxy.0.lock().expect("Poisoned");
            locked
                .send_event(UserEvent::MenuItem(window_id, mid))
                .expect("EventLoop closed");
        })?;

        // Create notify icon
        let notify_event_proxy = event_proxy.clone();
        let notify_icon = NotifyIcon::new(
            hwnd,
            NOTIFY_ICON_MESSAGE_ID,
            APPLICATION_TITLE,
            ICON_IMAGE_PNG,
            move |message, (x, y)| {
                let locked = notify_event_proxy.0.lock().expect("Poisoned");
                locked
                    .send_event(UserEvent::NotifyIconMessage(window_id, message, x, y))
                    .expect("EventLoop closed");
            },
        )?;

        // Create WGPU related objects
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Cannot initialize adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .context("Cannot initialize device")?;
        let surface_format = surface
            .get_preferred_format(&adapter)
            .context("Cannot determine surface format")?;
        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Fifo,
        };

        // Create egui related objects
        let egui_context = EguiContext::default();
        let egui_state = EguiState::new(4096, &window);
        let egui_render_pass = EguiRenderPass::new(&device, surface_format, 1);

        // Create application logic
        {
            let mut locked = application.lock().expect("Poisoned");
            locked.attach_event_loop(event_proxy.clone());
        }

        Ok(MainWindow {
            window,
            event_proxy,
            task_menu,
            _notify_icon: notify_icon,
            _instance: instance,
            _adapter: adapter,
            surface,
            device,
            queue,
            surface_config,
            egui_context,
            egui_state,
            egui_render_pass,
            application,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    /// Called after all events are processed.
    pub fn after_events(&mut self) {
        self.window.request_redraw();
    }

    pub fn on_window_event(&mut self, event: WindowEvent) -> Option<ControlFlow> {
        match event {
            WindowEvent::CloseRequested => {
                self.window.set_visible(false);
                None
            }
            WindowEvent::Resized(new_size) => {
                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                // See: https://github.com/rust-windowing/winit/issues/208
                // This solves an issue where the app would panic when minimizing on Windows.
                if new_size.width > 0 && new_size.height > 0 {
                    self.surface_config.width = new_size.width;
                    self.surface_config.height = new_size.height;
                    self.surface.configure(&self.device, &self.surface_config);
                }
                None
            }
            event => {
                self.egui_state.on_event(&self.egui_context, &event);
                None
            }
        }
    }

    /// Called when redraw is reqested for this window.
    pub fn on_redraw(&mut self) -> Option<ControlFlow> {
        let output_frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated) => return None,
            Err(err) => {
                error!("Failed to fetch texture: {err}");
                return None;
            }
        };
        let texture_view = output_frame.texture.create_view(&Default::default());

        // Update view
        let input = self.egui_state.take_egui_input(&self.window);
        let (commands, textures_delta, repainting) =
            self.draw_egui(input, self.window.scale_factor() as f32);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.surface_config.width,
            physical_height: self.surface_config.height,
            scale_factor: self.window.scale_factor() as f32,
        };

        // Transfer to GPU
        self.update_gpu_state(&screen_descriptor, &commands, textures_delta);
        self.transfer_to_gpu(&texture_view, &commands, &screen_descriptor);

        // Write back
        output_frame.present();
        if repainting {
            Some(ControlFlow::Poll)
        } else {
            Some(ControlFlow::Wait)
        }
    }

    pub fn on_menu_select(&self, mid: u32) {
        match mid {
            MENU_ID_SHOW => {
                self.window.set_visible(true);
            }
            MENU_ID_EXIT => {
                let locked = self.event_proxy.0.lock().expect("Poisoned");
                locked
                    .send_event(UserEvent::ExitRequested)
                    .expect("EventLoop closed");
            }
            _ => (),
        }
    }

    /// Called when related `NotifyIcon` triggered events.
    pub fn on_notify_icon(&self, msg: u16, x: i16, y: i16) {
        match msg as u32 {
            WM_LBUTTONUP => {
                self.window.set_visible(true);
            }
            WM_CONTEXTMENU => {
                self.task_menu.track_at(x as i32, y as i32);
            }
            _ => (),
        }
    }

    /// Repaint egui.
    fn draw_egui(
        &mut self,
        input: RawInput,
        scale_factor: f32,
    ) -> (Vec<ClippedMesh>, TexturesDelta, bool) {
        self.egui_context.begin_frame(input);

        let app_output = epi::backend::AppOutput::default();
        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: "egui_wgpu",
                web_info: None,
                cpu_usage: None,
                native_pixels_per_point: Some(scale_factor),
                prefer_dark_mode: None,
            },
            output: app_output,
            repaint_signal: self.event_proxy.clone(),
        });

        let mut locked = self.application.lock().expect("Posioned");
        locked.update(&self.egui_context, &frame);

        let full_output = self.egui_context.end_frame();
        let paint_jobs = self.egui_context.tessellate(full_output.shapes);
        (
            paint_jobs,
            full_output.textures_delta,
            full_output.needs_repaint,
        )
    }

    /// Uploads all information to the GPU.
    fn update_gpu_state(
        &mut self,
        descriptor: &ScreenDescriptor,
        commands: &[ClippedMesh],
        textures_delta: TexturesDelta,
    ) {
        self.egui_render_pass
            .add_textures(&self.device, &self.queue, &textures_delta)
            .unwrap();
        self.egui_render_pass
            .remove_textures(textures_delta)
            .unwrap();
        self.egui_render_pass
            .update_buffers(&self.device, &self.queue, commands, descriptor);
    }

    /// Sends commands to queue.
    fn transfer_to_gpu(
        &self,
        texture_view: &TextureView,
        commands: &[ClippedMesh],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let mut encoder = self.device.create_command_encoder(&ENCODER_DESCRIPTION);
        self.egui_render_pass
            .execute(
                &mut encoder,
                texture_view,
                commands,
                screen_descriptor,
                Some(wgpu::Color::BLACK),
            )
            .unwrap();
        self.queue.submit([encoder.finish()]);
    }
}
