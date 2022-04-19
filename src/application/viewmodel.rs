pub use crate::application::model::WallpaperListOperation;

use crate::{
    application::{
        model::{Application, ApplicationEvent},
        Fitting, Wallpaper,
    },
    mvvm::{EventManager, Observable, Subscription},
    windows::Monitor,
};

use std::sync::Arc;

use anyhow::Result;
use log::{error, info};
use native_dialog::FileDialog;
use parking_lot::Mutex;
use tokio::task::spawn_blocking;
use uuid::Uuid;
use vek::{Vec2, Vec4};

pub struct ApplicationViewModel {
    model: Arc<Mutex<Application>>,
    model_subscription: Option<Subscription<ApplicationEvent>>,
    subscriber_views: EventManager<ApplicationViewModelEvent>,

    pub monitors: Vec<MonitorCache>,
    pub wallpapers: Vec<WallpaperCache>,
}

impl ApplicationViewModel {
    /// Constructs new ViewModel.
    pub fn new(model: Arc<Mutex<Application>>) -> Arc<Mutex<ApplicationViewModel>> {
        let viewmodel = Arc::new(Mutex::new(ApplicationViewModel {
            model: model.clone(),
            model_subscription: None,
            subscriber_views: EventManager::new(),

            monitors: vec![],
            wallpapers: vec![],
        }));

        let subscription = ApplicationViewModel::setup_subscribe(model, viewmodel.clone());
        {
            let mut locked = viewmodel.lock();
            locked.model_subscription = Some(subscription);
        }

        viewmodel
    }

    /// Register the subscription for model event.
    fn setup_subscribe(
        model: Arc<Mutex<Application>>,
        viewmodel: Arc<Mutex<ApplicationViewModel>>,
    ) -> Subscription<ApplicationEvent> {
        let mut model = model.lock();

        let vm = viewmodel.clone();
        model.subscribe(move |e| match e {
            ApplicationEvent::MonitorsUpdated => {
                let vm = vm.clone();
                spawn_blocking(|| ApplicationViewModel::update_monitors(vm));
            }
            ApplicationEvent::WallpapersUpdated => {
                let vm = vm.clone();
                spawn_blocking(|| ApplicationViewModel::update_wallpapers(vm));
            }
        })
    }
}

/// ViewModel event handlers.
impl ApplicationViewModel {
    /// Calculates normalized monitor preview rects.
    /// Should be called as dedicated task.
    pub fn update_monitors(this: Arc<Mutex<ApplicationViewModel>>) {
        let mut viewmodel = this.lock();

        viewmodel.monitors.clear();
        let monitors_source = {
            let model = viewmodel.model.lock();
            model.monitors().to_vec()
        };
        if monitors_source.is_empty() {
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
            viewmodel.monitors.push(MonitorCache::new(
                &monitor,
                whole_topleft,
                whole_offset,
                divider,
            ));
        }

        viewmodel.notify(ApplicationViewModelEvent::MonitorsUpdated);
    }

    /// Updates wallpaper thumbnail cache.
    /// Should be called as dedicated task.
    pub fn update_wallpapers(this: Arc<Mutex<ApplicationViewModel>>) {
        let mut viewmodel = this.lock();

        let wallpapers_source = {
            let model = viewmodel.model.lock();
            model.wallpapers().to_vec()
        };

        viewmodel.wallpapers.clear();
        for wallpaper in wallpapers_source {
            let wv = WallpaperCache::new(&wallpaper);
            viewmodel.wallpapers.push(wv);
        }

        viewmodel.notify(ApplicationViewModelEvent::WallpapersUpdated);
    }
}

impl ApplicationViewModel {
    /// Opens file selection dialog.
    pub fn action_add_image(this: Arc<Mutex<ApplicationViewModel>>) -> Result<()> {
        let viewmodel = this.lock();

        let selected = FileDialog::new()
            .add_filter("Supported Image Files", &["jpg", "jpeg", "png", "bmp"])
            .show_open_single_file()
            .expect("Invalid file open dialog");
        let path = match selected {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut locked = viewmodel.model.lock();
        locked.add_wallpaper(Wallpaper::new(path.to_string_lossy(), Fitting::Cover));

        Ok(())
    }

    /// Performs wallpapers list operation.
    pub fn action_perform_wallpaper(
        this: Arc<Mutex<ApplicationViewModel>>,
        index: usize,
        op: WallpaperListOperation,
    ) {
        let viewmodel = this.lock();
        let mut locked = viewmodel.model.lock();
        locked.update_wallpaper(index, op);
    }

    /// Sets selected wallpaper.
    pub fn action_set_wallpaper(
        this: Arc<Mutex<ApplicationViewModel>>,
        monitor_index: usize,
        wallpaper_index: usize,
    ) {
        info!("Changing wallpaper: Monitor #{monitor_index}: Wallpaper #{wallpaper_index}");
        let viewmodel = this.lock();
        let locked = viewmodel.model.lock();
        match locked.apply_wallpaper_for_monitor(monitor_index, wallpaper_index) {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to set wallpaper: {e}");
            }
        }
    }
}

impl Observable for ApplicationViewModel {
    type Message = ApplicationViewModelEvent;

    fn subscribe<S>(&mut self, subscription: S) -> Subscription<ApplicationViewModelEvent>
    where
        S: Fn(ApplicationViewModelEvent) + Send + Sync + 'static,
    {
        self.subscriber_views.subscribe(subscription)
    }

    fn notify(&mut self, message: ApplicationViewModelEvent) {
        self.subscriber_views.notify(message);
    }
}

/// Represents an event in `ApplicationViewModel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationViewModelEvent {
    WallpapersUpdated,
    MonitorsUpdated,
}

/// Cache object for view about monitor.
pub struct MonitorCache {
    pub name: String,
    pub position: Vec2<i32>,
    pub size: Vec2<i32>,
    pub preview_rect: Vec4<f32>,
}

impl MonitorCache {
    /// Constructs from Monitor model.
    pub fn new(
        source: &Monitor,
        whole_topleft: Vec2<f32>,
        whole_offset: Vec2<f32>,
        divider: f32,
    ) -> MonitorCache {
        let raw_position = source.position().as_::<f32>();
        let raw_size = source.size().as_::<f32>();

        let normalized_position = (raw_position - whole_topleft + whole_offset) / divider;
        let normalized_size = raw_size / divider;

        MonitorCache {
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

/// Cache object for view about wallpaper.
pub struct WallpaperCache {
    pub uuid: Uuid,
    pub filename: String,
    pub fitting: Fitting,
}

impl WallpaperCache {
    pub fn new(source: &Wallpaper) -> WallpaperCache {
        WallpaperCache {
            uuid: source.id(),
            filename: source.filename().to_string(),
            fitting: source.fitting(),
        }
    }
}
