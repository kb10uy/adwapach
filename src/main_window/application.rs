use crate::{
    main_window::{EventProxy, UserEvent},
    windows::Monitor,
};

use std::sync::Arc;

use egui::{
    menu, text::LayoutJob, Align, CentralPanel, Color32, Context as EguiContext, Direction, FontId,
    Grid, Id, Layout, Pos2 as UiPos2, Rect, Response, RichText, ScrollArea, Sense, Stroke, Style,
    TextFormat, TextStyle, TopBottomPanel, Ui, Vec2 as UiVec2,
};
use epi::{App as EpiApp, Frame as EpiFrame, Storage as EpiStorage};
use vek::{Vec2, Vec4};

pub struct MainWindowApp {
    event_proxy: Option<Arc<EventProxy>>,
    monitors: Vec<Monitor>,
    monitor_preview_rects: Vec<Vec4<f32>>,
    selected_monitor_index: Option<usize>,
}

impl MainWindowApp {
    pub fn attach_event_loop(&mut self, proxy: Arc<EventProxy>) {
        self.event_proxy = Some(proxy);
    }

    pub fn set_monitors(&mut self, monitors: Vec<Monitor>) {
        self.monitors = monitors;
        self.selected_monitor_index = if self.monitors.is_empty() {
            None
        } else {
            Some(0)
        };

        self.calculate_preview();
    }

    /// Calculates normalized monitor preview rects.
    fn calculate_preview(&mut self) {
        self.monitor_preview_rects.clear();
        if self.monitors.is_empty() {
            return;
        }

        let (x_points, y_points): (Vec<_>, Vec<_>) = self
            .monitors
            .iter()
            .flat_map(|m| {
                let position = m.position();
                let size = m.size();
                [
                    (position.x, position.y),
                    (position.x + size.x, position.y + size.y),
                ]
            })
            .unzip();

        let left_all = *x_points.iter().min().expect("No monitor calculated") as f32;
        let right_all = *x_points.iter().max().expect("No monitor calculated") as f32;
        let top_all = *y_points.iter().min().expect("No monitor calculated") as f32;
        let bottom_all = *y_points.iter().max().expect("No monitor calculated") as f32;

        let position_all = Vec2::new(left_all, top_all);
        let size_all = Vec2::new(right_all - left_all, bottom_all - top_all);
        let divider = size_all.x.max(size_all.y);
        let offset = (Vec2::new(divider, divider) - size_all) / 2.0;

        for monitor in &self.monitors {
            let raw_position = monitor.position().as_::<f32>();
            let raw_size = monitor.size().as_::<f32>();

            let normalized_position = (raw_position - position_all + offset) / divider;
            let normalized_size = raw_size / divider;

            self.monitor_preview_rects.push(Vec4::new(
                normalized_position.x,
                normalized_position.y,
                normalized_position.x + normalized_size.x,
                normalized_position.y + normalized_size.y,
            ));
        }
    }
}

/// UI Actions
impl MainWindowApp {
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
            monitor_preview_rects: vec![],
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
            .insert(TextStyle::Body, FontId::proportional(16.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(18.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(20.0));

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

        let mut selected_index = match self.selected_monitor_index {
            Some(i) => i,
            None => return,
        };
        let selected_size = self.monitors[selected_index].size();
        let selected_position = self.monitors[selected_index].position();

        CentralPanel::default().show(ctx, |ui| {
            // Monitor preview & selection
            ui.vertical_centered(|ui| {
                self.ui_draw_monitor_preview(ui, 320.0, selected_index, &mut selected_index);
            });
            ui.horizontal_wrapped(|ui| {
                for (i, monitor) in self.monitors.iter().enumerate() {
                    ui.selectable_value(&mut selected_index, i, monitor.name())
                        .on_hover_text(monitor.id_as_string());
                }
            });
            ui.separator();
            self.selected_monitor_index = Some(selected_index);

            Grid::new("monitor_info")
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

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                for i in 0..10 {
                    if self.ui_draw_image_item(ui, i).double_clicked() {
                        println!("double-click {i}");
                    }
                }
            });
        });
    }
}

/// UI Elements
impl MainWindowApp {
    /// Draws monitor preview rects.
    fn ui_draw_monitor_preview(
        &self,
        ui: &mut Ui,
        size: f32,
        selected: usize,
        target: &mut usize,
    ) -> Response {
        let (response, painter) = ui.allocate_painter(UiVec2::new(size, size), Sense::hover());
        let rect_size = response.rect.size();
        let multiplier = response.rect.width().min(response.rect.height());
        let left_top = response.rect.left_top();
        let offset = left_top + (rect_size - UiVec2::splat(multiplier)) / 2.0;

        let stroke = Stroke::new(2.0, Color32::WHITE);
        let stroke_selected = Stroke::new(2.0, Color32::BLUE);
        let fill = Color32::from_white_alpha(32);
        for (i, monitor_preview) in self.monitor_preview_rects.iter().enumerate() {
            let mlt = UiPos2::new(
                monitor_preview.x * multiplier + 2.0,
                monitor_preview.y * multiplier + 2.0,
            ) + offset.to_vec2();
            let mrb = UiPos2::new(
                monitor_preview.z * multiplier - 2.0,
                monitor_preview.w * multiplier - 2.0,
            ) + offset.to_vec2();
            let monitor_rect = Rect::from_min_max(mlt, mrb);

            painter.rect_filled(monitor_rect, 2.0, fill);

            if i == selected {
                painter.rect_stroke(monitor_rect, 2.0, stroke_selected);
            } else {
                painter.rect_stroke(monitor_rect, 2.0, stroke);
            }

            let monitor_response = ui.interact(
                monitor_rect,
                Id::new(format!("monitor_preview_{i}")),
                Sense::click(),
            );
            if monitor_response.clicked() {
                *target = i;
            }
        }

        response
    }

    /// Draw an item of wallpaper image list.
    fn ui_draw_image_item(&self, ui: &mut Ui, i: usize) -> Response {
        let left_center_layout =
            Layout::centered_and_justified(Direction::TopDown).with_cross_align(Align::LEFT);
        let head_style = TextFormat {
            font_id: TextStyle::Heading.resolve(ui.style()),
            color: Color32::WHITE,
            ..Default::default()
        };
        let prop_style = TextFormat {
            font_id: TextStyle::Body.resolve(ui.style()),
            ..Default::default()
        };

        let inner_response = ui.horizontal(|ui| {
            ui.allocate_painter(UiVec2::splat(100.0), Sense::click());
            ui.with_layout(left_center_layout, |ui| {
                let mut text = LayoutJob::default();
                text.append(&format!("test{i}.jpg\n"), 0.0, head_style);
                text.append("Size: 1920x1080\n", 0.0, prop_style.clone());
                text.append("Fitting: Cover", 0.0, prop_style);
                ui.label(text);
            });
        });

        ui.interact(
            inner_response.response.rect,
            Id::new(format!("asdad{i}")),
            Sense::click(),
        )
    }
}
