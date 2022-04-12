use crate::{
    main_window::{EventProxy, UserEvent},
    windows::Monitor,
};

use std::sync::Arc;

use egui::{
    menu, CentralPanel, Context as EguiContext, FontId, Grid, RichText, Style, TextStyle,
    TopBottomPanel,
};
use epi::{App as EpiApp, Frame as EpiFrame, Storage as EpiStorage};

pub struct MainWindowApp {
    event_proxy: Option<Arc<EventProxy>>,
    monitors: Vec<Monitor>,
    selected_monitor_index: Option<usize>,
}

impl MainWindowApp {
    pub fn attach_event_loop(&mut self, proxy: Arc<EventProxy>) {
        self.event_proxy = Some(proxy);
    }

    pub fn set_monitors(&mut self, monitors: Vec<Monitor>) {
        self.monitors = monitors;
        self.selected_monitor_index = if self.monitors.len() > 0 {
            Some(0)
        } else {
            None
        };
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
        MainWindowApp {
            event_proxy: None,
            monitors: vec![],
            selected_monitor_index: None,
        }
    }
}

impl EpiApp for MainWindowApp {
    fn name(&self) -> &str {
        "Adwapach"
    }

    fn setup(&mut self, ctx: &EguiContext, _frame: &EpiFrame, _storage: Option<&dyn EpiStorage>) {
        let mut style = Style::default();
        style.override_font_id = Some(FontId::proportional(16.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(18.0));

        ctx.set_style(style);
    }

    fn update(&mut self, ctx: &EguiContext, _frame: &EpiFrame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("Application", |ui| {
                    if ui.button("Quit").clicked() {
                        self.request_exit();
                    }
                });
            });
        });

        let selected_monitor = match self.selected_monitor_index {
            Some(i) => &self.monitors[i],
            None => return,
        };
        let selected_size = selected_monitor.size();
        let selected_position = selected_monitor.position();

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, monitor) in self.monitors.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.selected_monitor_index,
                        Some(i),
                        format!("Monitor #{}", i + 1),
                    )
                    .on_hover_text(monitor.id_as_string());
                }
            });

            Grid::new("monitor_select")
                .num_columns(2)
                .min_col_width(128.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Position").strong())
                        .on_hover_text("Top-left position of monitor, relative to first");
                    ui.label(format!(
                        "X: {}, Y: {}",
                        selected_position.x, selected_position.y
                    ));
                    ui.end_row();

                    ui.label(RichText::new("Size").strong())
                        .on_hover_text("Phyisical monitor size");
                    ui.label(format!(
                        "Width: {}, Height: {}",
                        selected_size.x, selected_size.y
                    ));
                    ui.end_row();
                });

            egui::warn_if_debug_build(ui);
        });
    }
}
