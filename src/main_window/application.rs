use crate::main_window::EventProxy;

use std::sync::Arc;

use egui::{ClippedMesh, Context as EguiContext, RawInput, TexturesDelta};
use epi::App;

pub struct MainWindowContext {
    egui_context: EguiContext,
    event_proxy: Arc<EventProxy>,
    demo_app: egui_demo_lib::WrapApp,
}

impl MainWindowContext {
    pub fn new(event_proxy: Arc<EventProxy>) -> MainWindowContext {
        let egui_context = EguiContext::default();
        let demo_app = egui_demo_lib::WrapApp::default();

        MainWindowContext {
            egui_context,
            event_proxy,
            demo_app,
        }
    }

    pub fn context(&self) -> &EguiContext {
        &self.egui_context
    }

    pub fn draw(
        &mut self,
        input: RawInput,
        scale_factor: f32,
    ) -> (Vec<ClippedMesh>, TexturesDelta, bool) {
        self.egui_context.begin_frame(input);

        let app_output = epi::backend::AppOutput::default();
        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: "egui_example",
                web_info: None,
                cpu_usage: None,
                native_pixels_per_point: Some(scale_factor),
                prefer_dark_mode: None,
            },
            output: app_output,
            repaint_signal: self.event_proxy.clone(),
        });

        self.demo_app.update(&self.egui_context, &frame);

        let full_output = self.egui_context.end_frame();
        let paint_jobs = self.egui_context.tessellate(full_output.shapes);
        (
            paint_jobs,
            full_output.textures_delta,
            full_output.needs_repaint,
        )
    }
}
