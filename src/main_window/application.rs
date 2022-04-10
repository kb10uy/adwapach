use crate::main_window::{EventProxy, UserEvent};

use std::sync::Arc;

use egui::Context as EguiContext;
use epi::{App as EpiApp, Frame as EpiFrame};

pub struct MainWindowApp {
    event_proxy: Option<Arc<EventProxy>>,
}

impl MainWindowApp {
    pub fn attach_event_loop(&mut self, proxy: Arc<EventProxy>) {
        self.event_proxy = Some(proxy);
    }

    /// Tells the window to quit.
    pub fn request_exit(&self) {
        if let Some(proxy) = &self.event_proxy {
            proxy
                .0
                .lock()
                .expect("Poisoned")
                .send_event(UserEvent::ExitRequested)
                .expect("EventLoop closed");
        }
    }
}

impl Default for MainWindowApp {
    fn default() -> MainWindowApp {
        MainWindowApp { event_proxy: None }
    }
}

impl EpiApp for MainWindowApp {
    fn update(&mut self, ctx: &EguiContext, _frame: &EpiFrame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Application", |ui| {
                    if ui.button("Quit").clicked() {
                        self.request_exit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });
    }

    fn name(&self) -> &str {
        "Adwapach"
    }
}
