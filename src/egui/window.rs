use crate::view::{EventProxy, RepaintableEvent, View};

use std::sync::Arc;

use anyhow::{Context, Result};
use egui::{ClippedMesh, Context as EguiContext, RawInput, TexturesDelta};
use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit::State as EguiState;
use epi::{
    backend::{AppOutput, FrameData},
    Frame as EpiFrame, IntegrationInfo,
};
use log::error;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, SurfaceError, TextureView};
use winit::{
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowBuilderExtWindows,
    window::{Window, WindowBuilder, WindowId},
};

const ENCODER_DESCRIPTION: wgpu::CommandEncoderDescriptor = wgpu::CommandEncoderDescriptor {
    label: Some("Egui Encoder"),
};

pub struct EguiWindow<V: View<E>, E: RepaintableEvent> {
    window: Window,
    event_proxy: Arc<EventProxy<E>>,
    surface: Surface,
    device: Device,
    queue: Queue,
    surface_config: SurfaceConfiguration,
    egui_context: EguiContext,
    egui_state: EguiState,
    egui_render_pass: EguiRenderPass,
    egui_base_frame: EpiFrame,
    view: V,
}

impl<V: View<E>, E: RepaintableEvent> EguiWindow<V, E> {
    pub async fn create(event_loop: &EventLoop<E>, mut view: V) -> Result<EguiWindow<V, E>> {
        let event_proxy = EventProxy::new(event_loop);

        // Create window
        let window = WindowBuilder::new()
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_drag_and_drop(false)
            .with_inner_size(LogicalSize::new(640, 640))
            .with_window_icon(view.get_icon())
            .with_title(view.name())
            .build(event_loop)?;

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
        let egui_base_frame = EpiFrame::new(FrameData {
            info: IntegrationInfo {
                name: "egui_wgpu",
                native_pixels_per_point: Some(window.scale_factor() as f32),
                web_info: None,
                cpu_usage: None,
                prefer_dark_mode: None,
            },
            output: AppOutput::default(),
            repaint_signal: event_proxy.clone(),
        });

        // Create application logic
        view.attach_window(&window, event_proxy.clone());
        view.setup(&egui_context, &egui_base_frame, None);

        Ok(EguiWindow {
            window,
            event_proxy,
            surface,
            device,
            queue,
            surface_config,
            egui_context,
            egui_state,
            egui_render_pass,
            egui_base_frame,
            view,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    /// Should call after all events are cleared.
    pub fn on_event_cleared(&self) {
        self.window.request_redraw();
    }

    /// Sets visibility.
    pub fn set_visibility(&self, visibility: bool) {
        self.window.set_visible(visibility);
    }

    /// Updates UI with arrived event.
    pub fn update_with_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.window.set_visible(false);
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
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let mut frame_data = self.egui_base_frame.0.lock().expect("epi::Frame poisoned");
                frame_data.info.native_pixels_per_point = Some(scale_factor as f32);
            }
            event => {
                self.egui_state.on_event(&self.egui_context, &event);
            }
        }
    }

    /// Redraws UI.
    pub fn redraw(&mut self) -> ControlFlow {
        let output_frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(SurfaceError::Outdated) => {
                // Window is minimized
                return ControlFlow::Poll;
            }
            Err(err) => {
                error!("Failed to fetch texture: {err}");
                return ControlFlow::Poll;
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
            ControlFlow::Poll
        } else {
            ControlFlow::Wait
        }
    }

    /// Repaint egui.
    fn draw_egui(
        &mut self,
        input: RawInput,
        scale_factor: f32,
    ) -> (Vec<ClippedMesh>, TexturesDelta, bool) {
        self.egui_context.begin_frame(input);

        let frame = self.egui_base_frame.clone();
        let mut frame_data = frame.0.lock().expect("Posioned");
        frame_data.info.native_pixels_per_point = Some(scale_factor);
        drop(frame_data);

        self.view.update(&self.egui_context, &frame);

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
