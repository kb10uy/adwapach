use crate::{
    model::{
        application::{Application, ApplicationEvent, Fitting, Wallpaper, WallpaperListOperation},
        Observable, Subscription,
    },
    view::{EguiEvent, EventProxy, View},
    windows::{MenuItem, Monitor, NotifyIcon, PopupMenu},
};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use egui::{
    menu, text::LayoutJob, Align, CentralPanel, Color32, ColorImage, Context, Direction, FontId,
    Grid, Id, Layout, Pos2 as UiPos2, Rect, Response, RichText, ScrollArea, Sense, Stroke, Style,
    TextFormat, TextStyle, TextureHandle, TopBottomPanel, Ui, Vec2 as UiVec2,
};
use epi::{App, Frame, Storage};
use image::{imageops::FilterType, DynamicImage, ImageBuffer};
use log::{error, info};
use native_dialog::FileDialog;
use parking_lot::Mutex;
use tokio::task::spawn_blocking;
use uuid::Uuid;
use vek::{Vec2, Vec4};
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

/// Main application view.
pub struct ApplicationView {
    model: Arc<Mutex<Application>>,
    subscription: Option<Subscription<ApplicationEvent>>,
    event_proxy: Option<Arc<EventProxy<ApplicationViewEvent>>>,
    notify_icon: Option<NotifyIcon>,
    context: Option<Context>,

    monitors: Vec<MonitorView>,
    selected_monitor_index: Option<usize>,

    wallpapers: Vec<WallpaperView>,
    wallpaper_cache: HashMap<Uuid, (TextureHandle, Vec2<u32>)>,
}

impl ApplicationView {
    pub fn new(model: Arc<Mutex<Application>>) -> Result<Arc<Mutex<ApplicationView>>> {
        let view = Arc::new(Mutex::new(ApplicationView {
            model: model.clone(),
            subscription: None,
            event_proxy: None,
            notify_icon: None,
            context: None,
            monitors: vec![],
            selected_monitor_index: None,
            wallpapers: vec![],
            wallpaper_cache: Default::default(),
        }));

        let subscription = ApplicationView::setup_subscribe(model, view.clone());
        {
            let mut locked = view.lock();
            locked.subscription = Some(subscription);
        }

        Ok(view)
    }

    /// Register the subscription for model event.
    fn setup_subscribe(
        model: Arc<Mutex<Application>>,
        view: Arc<Mutex<ApplicationView>>,
    ) -> Subscription<ApplicationEvent> {
        let mut model = model.lock();

        let model_view = view.clone();
        model.subscribe(move |e| match e {
            ApplicationEvent::MonitorsUpdated => {
                let view = model_view.clone();
                spawn_blocking(|| ApplicationView::vm_update_monitors(view));
            }
            ApplicationEvent::WallpapersUpdated => {
                let view = model_view.clone();
                spawn_blocking(|| ApplicationView::vm_reconstruct_wallpaper_cache(view));
            }
        })
    }
}

/// ViewModel async events.
impl ApplicationView {
    /// Calculates normalized monitor preview rects.
    /// Should be called as dedicated task.
    fn vm_update_monitors(this: Arc<Mutex<ApplicationView>>) {
        let mut view = this.lock();

        view.monitors.clear();
        let monitors_source = {
            let model = view.model.lock();
            model.monitors().to_vec()
        };
        if monitors_source.is_empty() {
            view.selected_monitor_index = None;
            return;
        }

        let (x_points, y_points): (Vec<_>, Vec<_>) = monitors_source
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

        let whole_topleft = Vec2::new(left_all, top_all);
        let whole_size = Vec2::new(right_all - left_all, bottom_all - top_all);
        let divider = whole_size.x.max(whole_size.y);
        let whole_offset = (Vec2::new(divider, divider) - whole_size) / 2.0;

        for monitor in monitors_source {
            view.monitors.push(MonitorView::new(
                &monitor,
                whole_topleft,
                whole_offset,
                divider,
            ));
        }

        view.selected_monitor_index = Some(0);
    }

    /// Updates wallpaper thumbnail cache.
    /// Should be called as dedicated task.
    fn vm_reconstruct_wallpaper_cache(this: Arc<Mutex<ApplicationView>>) {
        let wallpapers_source = {
            let view = this.lock();
            let model = view.model.lock();
            model.wallpapers().to_vec()
        };
        let mut active_thumbnails = HashSet::new();
        let mut unmet_thumbnails = vec![];

        // Just push metadata
        {
            let mut view = this.lock();
            view.wallpapers.clear();
            for (i, wallpaper) in wallpapers_source.iter().enumerate() {
                let mut wv = WallpaperView::new(&wallpaper);

                match view.wallpaper_cache.get(&wallpaper.id()) {
                    Some(cache) => {
                        wv.size = Some(cache.1);
                        active_thumbnails.insert(wallpaper.id());
                    }
                    None => {
                        unmet_thumbnails.push((i, wallpaper.filename().to_string()));
                    }
                }
                view.wallpapers.push(wv);
            }
        }

        // Load unmet files
        for (index, filename) in unmet_thumbnails {
            info!("Loading {filename}");
            let (mut resized_image, original_size) = match image::open(&filename) {
                Ok(i) => {
                    let size = Vec2::new(i.width(), i.height());
                    let resized_image = i.resize(512, 512, FilterType::Gaussian);
                    (resized_image, size)
                }
                Err(e) => {
                    error!("Image load error: {e}");
                    let placeholder = DynamicImage::ImageRgba8(ImageBuffer::new(128, 128));
                    (placeholder, Vec2::new(0, 0))
                }
            };
            let rect_size = resized_image.width().min(resized_image.height());
            resized_image = resized_image.crop(
                (resized_image.width() - rect_size) / 2,
                (resized_image.height() - rect_size) / 2,
                rect_size,
                rect_size,
            );

            let ui_image = ColorImage::from_rgba_unmultiplied(
                [rect_size as _, rect_size as _],
                &resized_image.to_rgba8(),
            );

            {
                let mut view = this.lock();
                let texture_handle = view
                    .context
                    .as_ref()
                    .expect("Should attached")
                    .load_texture(&filename, ui_image);

                let id = view.wallpapers[index].uuid;
                view.wallpapers[index].size = Some(original_size);
                view.wallpaper_cache
                    .insert(id, (texture_handle, original_size));
                active_thumbnails.insert(id);
            }
        }

        // Delete unused
        {
            let mut view = this.lock();
            view.wallpaper_cache
                .retain(|k, _| active_thumbnails.contains(k));
        }
    }
}

/// View events.
impl View<ApplicationViewEvent> for ApplicationView {
    fn attach_window(
        &mut self,
        window: &Window,
        event_proxy: Arc<EventProxy<ApplicationViewEvent>>,
    ) {
        let window_id = window.id();
        let hwnd = HWND(window.hwnd() as _);

        // Create popup menu
        let menu_event_proxy = event_proxy.clone();
        let task_menu = PopupMenu::new(hwnd, TASK_MENU_ITEMS, move |mid| match mid {
            MENU_ID_SHOW => menu_event_proxy.request_show(window_id),
            MENU_ID_EXIT => menu_event_proxy.exit(),
            _ => (),
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
                WM_LBUTTONUP => notify_event_proxy.request_show(window_id),
                WM_CONTEXTMENU => task_menu.track_at(x as i32, y as i32),
                _ => (),
            },
        )
        .expect("Failed to register taskbar icon");

        self.notify_icon = Some(notify_icon);
        self.event_proxy = Some(event_proxy);
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

    fn setup(&mut self, ctx: &Context, _frame: &Frame, _storage: Option<&dyn Storage>) {
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

        self.context = Some(ctx.clone());
    }

    fn update(&mut self, ctx: &Context, _frame: &Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("Application", |ui| {
                    if ui.button("Quit").clicked() {
                        self.event_proxy
                            .as_ref()
                            .expect("Should have window")
                            .exit();
                    }
                });
            });
        });

        let mut selected_index = match self.selected_monitor_index {
            Some(i) => i,
            None => return,
        };
        let selected_size = self.monitors[selected_index].size;
        let selected_position = self.monitors[selected_index].position;

        CentralPanel::default().show(ctx, |ui| {
            // Monitor preview & selection
            ui.vertical_centered(|ui| {
                self.ui_draw_monitor_preview(ui, 320.0, selected_index, &mut selected_index);
            });
            ui.horizontal_wrapped(|ui| {
                for (i, monitor) in self.monitors.iter().enumerate() {
                    ui.selectable_value(&mut selected_index, i, &monitor.name);
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

            ui.horizontal_wrapped(|ui| {
                if ui.button("Add Image").clicked() {
                    let model = self.model.clone();
                    spawn_blocking(|| ApplicationView::action_add_image(model));
                }
            });

            ui.add_space(0.0);

            ScrollArea::vertical().show(ui, |ui| {
                self.ui_draw_image_items(ui);
            });
        });
    }
}

/// Sub-UI functions.
impl ApplicationView {
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

        for (i, monitor) in self.monitors.iter().enumerate() {
            let mlt = UiPos2::new(
                monitor.preview_rect.x * multiplier + 2.0,
                monitor.preview_rect.y * multiplier + 2.0,
            ) + offset.to_vec2();
            let mrb = UiPos2::new(
                monitor.preview_rect.z * multiplier - 2.0,
                monitor.preview_rect.w * multiplier - 2.0,
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
    fn ui_draw_image_items(&mut self, ui: &mut Ui) {
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
        let thumbnail_size = UiVec2::splat(100.0);

        for (i, wallpaper) in self.wallpapers.iter().enumerate() {
            let thumbnail = self.wallpaper_cache.get(&wallpaper.uuid);

            let inner_response = ui.horizontal(|ui| {
                match thumbnail {
                    Some((t, _)) => {
                        ui.image(t.id(), thumbnail_size);
                    }
                    None => {
                        ui.allocate_painter(thumbnail_size, Sense::hover());
                    }
                }

                ui.with_layout(left_center_layout, |ui| {
                    let mut text = LayoutJob::default();
                    text.append(
                        &format!("{}\n", wallpaper.filename),
                        0.0,
                        head_style.clone(),
                    );
                    match wallpaper.size {
                        Some(size) => text.append(
                            &format!("Size: {}x{}\n", size.x, size.y),
                            0.0,
                            prop_style.clone(),
                        ),
                        None => text.append("Size: Unknown\n", 0.0, prop_style.clone()),
                    }
                    text.append(
                        &format!("Fitting: {:?}", wallpaper.fitting),
                        0.0,
                        prop_style.clone(),
                    );
                    ui.label(text);
                });
            });

            let response = ui
                .interact(
                    inner_response.response.rect,
                    Id::new(format!("wallpaper_item_{i}")),
                    Sense::click(),
                )
                .context_menu(|ui| {
                    let mut selected_fitting = wallpaper.fitting;
                    ui.menu_button("Change Fitting", |ui| {
                        ui.selectable_value(&mut selected_fitting, Fitting::Cover, "Cover");
                        ui.selectable_value(&mut selected_fitting, Fitting::Contain, "Contain");
                        ui.selectable_value(&mut selected_fitting, Fitting::Tile, "Tile");
                        ui.selectable_value(&mut selected_fitting, Fitting::Center, "Center");
                    });
                    if selected_fitting != wallpaper.fitting {
                        let model = self.model.clone();
                        spawn_blocking(move || {
                            ApplicationView::action_perform_wallpaper(
                                model,
                                i,
                                WallpaperListOperation::SetFitting(selected_fitting),
                            )
                        });
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Move Up").clicked() {
                        let model = self.model.clone();
                        spawn_blocking(move || {
                            ApplicationView::action_perform_wallpaper(
                                model,
                                i,
                                WallpaperListOperation::MoveUp,
                            )
                        });
                        ui.close_menu();
                    }
                    if ui.button("Move Down").clicked() {
                        let model = self.model.clone();
                        spawn_blocking(move || {
                            ApplicationView::action_perform_wallpaper(
                                model,
                                i,
                                WallpaperListOperation::MoveDown,
                            )
                        });
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Remove").clicked() {
                        let model = self.model.clone();
                        spawn_blocking(move || {
                            ApplicationView::action_perform_wallpaper(
                                model,
                                i,
                                WallpaperListOperation::Remove,
                            )
                        });
                        ui.close_menu();
                    }
                });

            if response.double_clicked() {
                let selected = self.selected_monitor_index.expect("Should have monitor");
                let model = self.model.clone();
                spawn_blocking(move || ApplicationView::action_set_wallpaper(model, selected, i));
            }
        }
    }
}

/// UI Actions.
impl ApplicationView {
    /// Opens file selection dialog.
    fn action_add_image(model: Arc<Mutex<Application>>) -> Result<()> {
        let selected = FileDialog::new()
            .add_filter("Supported Image Files", &["jpg", "jpeg", "png", "bmp"])
            .show_open_single_file()
            .expect("Invalid file open dialog");
        let path = match selected {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut locked = model.lock();
        locked.add_wallpaper(Wallpaper::new(path.to_string_lossy(), Fitting::Cover));

        Ok(())
    }

    /// Performs wallpapers list operation.
    fn action_perform_wallpaper(
        model: Arc<Mutex<Application>>,
        index: usize,
        op: WallpaperListOperation,
    ) {
        let mut locked = model.lock();
        locked.update_wallpaper(index, op);
    }

    /// Sets selected wallpaper.
    fn action_set_wallpaper(
        model: Arc<Mutex<Application>>,
        monitor_index: usize,
        wallpaper_index: usize,
    ) {
        info!("Changing wallpaper: Monitor #{monitor_index}: Wallpaper #{wallpaper_index}");
        let locked = model.lock();
        match locked.apply_wallpaper_for_monitor(monitor_index, wallpaper_index) {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to set wallpaper: {e}");
            }
        }
    }
}

pub struct MonitorView {
    name: String,
    position: Vec2<i32>,
    size: Vec2<i32>,
    preview_rect: Vec4<f32>,
}

impl MonitorView {
    /// Constructs from Monitor model.
    pub fn new(
        source: &Monitor,
        whole_topleft: Vec2<f32>,
        whole_offset: Vec2<f32>,
        divider: f32,
    ) -> MonitorView {
        let raw_position = source.position().as_::<f32>();
        let raw_size = source.size().as_::<f32>();

        let normalized_position = (raw_position - whole_topleft + whole_offset) / divider;
        let normalized_size = raw_size / divider;

        MonitorView {
            name: source.name().to_string(),
            position: source.position(),
            size: source.size(),
            preview_rect: Vec4::new(
                normalized_position.x,
                normalized_position.y,
                normalized_position.x + normalized_size.x,
                normalized_position.y + normalized_size.y,
            ),
        }
    }
}

pub struct WallpaperView {
    uuid: Uuid,
    filename: String,
    size: Option<Vec2<u32>>,
    fitting: Fitting,
}

impl WallpaperView {
    pub fn new(source: &Wallpaper) -> WallpaperView {
        WallpaperView {
            uuid: source.id(),
            filename: source.filename().to_string(),
            size: None,
            fitting: source.fitting(),
        }
    }
}

/// User event type for `Application`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationViewEvent {
    Exit,
    RepaintRequested,
    ShowRequested(WindowId),
    HideRequested(WindowId),
}

impl EguiEvent for ApplicationViewEvent {
    fn repaint() -> ApplicationViewEvent {
        ApplicationViewEvent::RepaintRequested
    }

    fn show_window(window_id: winit::window::WindowId) -> Self {
        ApplicationViewEvent::ShowRequested(window_id)
    }

    fn hide_window(window_id: winit::window::WindowId) -> Self {
        ApplicationViewEvent::HideRequested(window_id)
    }

    fn exit() -> Self {
        ApplicationViewEvent::Exit
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

    fn should_exit(&self) -> bool {
        *self == ApplicationViewEvent::Exit
    }
}
