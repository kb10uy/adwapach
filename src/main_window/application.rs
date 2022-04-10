use egui::Context as EguiContext;
use epi::{App as EpiApp, Frame as EpiFrame};

pub struct MainWindowApp {
    pub should_exit: bool,
}

impl Default for MainWindowApp {
    fn default() -> MainWindowApp {
        MainWindowApp { should_exit: false }
    }
}

impl EpiApp for MainWindowApp {
    fn update(&mut self, ctx: &EguiContext, _frame: &EpiFrame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Application", |ui| {
                    self.should_exit = ui.button("Quit").clicked();
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
