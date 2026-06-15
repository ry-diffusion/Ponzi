use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod pressure_test;
pub mod tabs;
mod theme;

use ponzi_driver::config::Config;
use ponzi_driver::protocol::PenData;

const PROFILES_DIR: &str = "profiles";
const DEFAULT_PROFILE: &str = "default";

fn profiles_dir() -> PathBuf {
    let dirs: [PathBuf; 2] = [
        PathBuf::from(PROFILES_DIR),
        PathBuf::from("/etc/ponzi/profiles"),
    ];
    for d in &dirs {
        if d.exists() {
            return d.clone();
        }
    }
    let d = PathBuf::from(PROFILES_DIR);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn list_profiles() -> Vec<String> {
    let dir = profiles_dir();
    let mut profiles = vec![DEFAULT_PROFILE.to_string()];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                let name = name.to_string();
                if name != DEFAULT_PROFILE && entry.path().extension().map_or(false, |e| e == "toml") {
                    profiles.push(name);
                }
            }
        }
    }
    profiles.sort();
    profiles.dedup();
    profiles
}

fn profile_path(name: &str) -> PathBuf {
    if name == DEFAULT_PROFILE {
        let legacy = Path::new("config.toml");
        if legacy.exists() { return legacy.to_path_buf(); }
        profiles_dir().join("default.toml")
    } else {
        profiles_dir().join(format!("{}.toml", name))
    }
}

fn load_profile(name: &str) -> Config {
    let path = profile_path(name);
    Config::load(&path).unwrap_or_default()
}

fn save_profile(name: &str, config: &Config) {
    let path = profile_path(name);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&path, config.to_toml()) {
        Ok(()) => log::debug!("auto-saved profile '{}' to {}", name, path.display()),
        Err(e) => log::error!("failed to save profile '{}': {}", name, e),
    }
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("ponzi_ui=debug,ponzi_driver=debug,warn")
    ).init();

    ctrlc::set_handler(move || {
        log::info!("SIGINT received, exiting");
        std::process::exit(0);
    }).ok();

    // Start tray icon in background
    let _tray_handle = thread::spawn(|| {
        if let Err(e) = start_tray() {
            log::warn!("tray icon not available: {}", e);
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Ponzi"),
        ..Default::default()
    };

    eframe::run_native(
        "Ponzi",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(PonziApp::new(cc)))
        }),
    )
}

fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
    struct PonziTray;
    impl ksni::Tray for PonziTray {
        fn title(&self) -> String { "Ponzi".into() }
        fn icon_name(&self) -> String { "input-tablet".into() }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            vec![
                ksni::MenuItem::Standard(ksni::menu::StandardItem {
                    label: "Quit".into(),
                    activate: Box::new(|_| std::process::exit(0)),
                    ..Default::default()
                }),
            ]
        }
    }
    let service = ksni::TrayService::new(PonziTray);
    service.run();
    Ok(())
}

#[derive(Clone, Default)]
pub struct LiveData {
    pub pen: PenData,
    pub connected: bool,
    pub locked: bool,
    pub error_msg: String,
    pub pressure_history: Vec<f32>,
}

pub struct PonziApp {
    config: Config,
    prev_config_hash: u64,
    shared_config: Arc<Mutex<Config>>,
    current_profile: String,
    profiles: Vec<String>,
    live: Arc<Mutex<LiveData>>,
    lock_signal: Arc<Mutex<Option<bool>>>,
    active_tab: Tab,
    status_msg: String,
    status_timer: f64,
    pressure_test: pressure_test::PressureTest,
    pub automapper: tabs::AutoMapper,
}

fn config_hash(cfg: &Config) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let toml = cfg.to_toml();
    let mut h = DefaultHasher::new();
    toml.hash(&mut h);
    h.finish()
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Status,
    Mapping,
    Orientation,
    Pressure,
    Smoothing,
    Buttons,
    PressureTest,
}

impl Tab {
    fn icon(&self) -> &'static str {
        match self {
            Self::Status => "◈",
            Self::Mapping => "⊞",
            Self::Orientation => "↻",
            Self::Pressure => "◉",
            Self::Smoothing => "〰",
            Self::Buttons => "⌨",
            Self::PressureTest => "✎",
        }
    }
    fn label(&self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Mapping => "Mapping",
            Self::Orientation => "Orientation",
            Self::Pressure => "Pressure",
            Self::Smoothing => "Smoothing",
            Self::Buttons => "Buttons",
            Self::PressureTest => "Test",
        }
    }
    const ALL: &[Tab] = &[
        Self::Status, Self::Mapping, Self::Orientation,
        Self::Pressure, Self::Smoothing, Self::Buttons, Self::PressureTest,
    ];
}

impl PonziApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let current_profile = DEFAULT_PROFILE.to_string();
        let config = load_profile(&current_profile);
        let profiles = list_profiles();
        let prev_config_hash = config_hash(&config);

        let shared_config = Arc::new(Mutex::new(config.clone()));
        let live = Arc::new(Mutex::new(LiveData::default()));
        let lock_signal: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
        let live_clone = Arc::clone(&live);
        let lock_clone = Arc::clone(&lock_signal);
        let cfg_clone = Arc::clone(&shared_config);
        let ctx = cc.egui_ctx.clone();
        let vid = config.device.vendor_id;
        let pid = config.device.product_id;

        log::info!("starting USB thread for {:04X}:{:04X}", vid, pid);
        thread::spawn(move || usb_reader_thread(vid, pid, live_clone, lock_clone, ctx, cfg_clone));

        Self {
            config,
            prev_config_hash,
            shared_config,
            current_profile,
            profiles,
            live,
            lock_signal,
            active_tab: Tab::Status,
            status_msg: String::new(),
            status_timer: 0.0,
            pressure_test: pressure_test::PressureTest::new(),
            automapper: tabs::AutoMapper::default(),
        }
    }

    fn auto_save(&mut self) {
        let h = config_hash(&self.config);
        if h != self.prev_config_hash {
            self.prev_config_hash = h;
            save_profile(&self.current_profile, &self.config);
        }
    }

    fn switch_profile(&mut self, name: &str) {
        self.config = load_profile(name);
        self.current_profile = name.to_string();
        self.prev_config_hash = config_hash(&self.config);
        log::info!("switched to profile '{}'", name);
        self.status_msg = format!("Profile: {}", name);
        self.status_timer = 2.0;
    }
}

impl eframe::App for PonziApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let live = self.live.lock().unwrap().clone();

        // Sync UI config → driver + auto-save
        *self.shared_config.lock().unwrap() = self.config.clone();
        self.auto_save();

        if self.status_timer > 0.0 {
            self.status_timer -= ctx.input(|i| i.predicted_dt) as f64;
            if self.status_timer <= 0.0 {
                self.status_msg.clear();
            }
        }

        // ── Sidebar ──
        egui::SidePanel::left("sidebar")
            .exact_width(theme::SIDEBAR_W)
            .frame(theme::sidebar_frame())
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("PONZI").color(theme::ACCENT).size(18.0).strong());
                    ui.label(egui::RichText::new(env!("CARGO_PKG_VERSION")).color(theme::TEXT_DIM).size(10.0));
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    let (color, text) = if live.connected && !live.locked {
                        (theme::GREEN, "Active")
                    } else if live.connected {
                        (theme::YELLOW, "Paused")
                    } else {
                        (theme::RED_DIM, "Disconnected")
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    ui.label(egui::RichText::new(text).color(color).size(11.0));
                });

                if !live.error_msg.is_empty() && !live.connected {
                    ui.label(egui::RichText::new(&live.error_msg).color(theme::ORANGE).size(9.5));
                }

                ui.add_space(8.0);

                // Profile selector
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Profile").color(theme::TEXT_DIM).size(10.0));
                    let current = self.current_profile.clone();
                    egui::ComboBox::from_id_salt("profile_selector")
                        .selected_text(&current)
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for profile in &self.profiles.clone() {
                                if ui.selectable_label(*profile == current, profile).clicked() {
                                    self.switch_profile(profile);
                                }
                            }
                        });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(8.0);

                // Nav items
                for &tab in Tab::ALL {
                    let selected = self.active_tab == tab;
                    let resp = ui.allocate_ui(egui::vec2(ui.available_width(), 28.0), |ui| {
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 28.0),
                            egui::Sense::click(),
                        );
                        let hovered = resp.hovered();

                        if selected {
                            ui.painter().rect_filled(rect, 3, theme::ACCENT);
                            let rail = egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height()));
                            ui.painter().rect_filled(rail, 1, theme::ACCENT);
                        } else if hovered {
                            ui.painter().rect_filled(rect, 3, theme::BG_ELEVATED);
                        }

                        let text_color = if selected { theme::BG_DEEP } else if hovered { theme::ACCENT } else { theme::TEXT_SECONDARY };

                        let icon_pos = egui::pos2(rect.left() + 14.0, rect.center().y);
                        ui.painter().text(icon_pos, egui::Align2::CENTER_CENTER,
                            tab.icon(), egui::FontId::proportional(14.0), text_color);

                        let label_pos = egui::pos2(rect.left() + 34.0, rect.center().y);
                        ui.painter().text(label_pos, egui::Align2::LEFT_CENTER,
                            tab.label(), egui::FontId::proportional(12.5), text_color);

                        resp
                    });

                    if resp.inner.clicked() {
                        self.active_tab = tab;
                    }
                }

                // Bottom section
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    if !self.status_msg.is_empty() {
                        ui.label(egui::RichText::new(&self.status_msg).color(theme::ACCENT).size(10.0));
                        ui.add_space(4.0);
                    }

                    // New profile button
                    ui.horizontal(|ui| {
                        let new_btn = egui::Button::new(
                            egui::RichText::new("+ New Profile").color(theme::TEXT_SECONDARY).size(11.0)
                        ).fill(theme::BG_ELEVATED).corner_radius(3);
                        if ui.add_sized([ui.available_width() / 2.0 - 4.0, 24.0], new_btn).clicked() {
                            let name = format!("profile_{}", self.profiles.len());
                            save_profile(&name, &self.config);
                            self.profiles = list_profiles();
                            self.switch_profile(&name);
                        }

                        let reset_btn = egui::Button::new(
                            egui::RichText::new("Reset").color(theme::TEXT_SECONDARY).size(11.0)
                        ).fill(theme::BG_ELEVATED).corner_radius(3);
                        if ui.add_sized([ui.available_width(), 24.0], reset_btn).clicked() {
                            self.config = Config::default();
                            self.status_msg = "Reset to defaults".into();
                            self.status_timer = 2.0;
                        }
                    });

                    ui.add_space(6.0);

                    if live.connected && !live.locked {
                        let btn = egui::Button::new(
                            egui::RichText::new("⏸ Pause").color(theme::ORANGE).size(12.0)
                        ).fill(Color32::from_rgb(40, 32, 16)).corner_radius(3)
                         .stroke(Stroke::new(1.0, theme::ORANGE));
                        if ui.add_sized([ui.available_width(), 30.0], btn).clicked() {
                            log::info!("user requested PAUSE");
                            *self.lock_signal.lock().unwrap() = Some(true);
                        }
                    } else if live.connected && live.locked {
                        let btn = egui::Button::new(
                            egui::RichText::new("▶ Resume").color(theme::GREEN).size(12.0)
                        ).fill(Color32::from_rgb(16, 36, 24)).corner_radius(3)
                         .stroke(Stroke::new(1.0, theme::GREEN));
                        if ui.add_sized([ui.available_width(), 30.0], btn).clicked() {
                            log::info!("user requested RESUME");
                            *self.lock_signal.lock().unwrap() = Some(false);
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                });
            });

        // ── Main content ──
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(theme::BG_PANEL).inner_margin(egui::Margin::same(16)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        Tab::Status => tabs::status_tab(ui, &live, &self.config),
                        Tab::Mapping => tabs::mapping_tab(ui, &mut self.config.mapping, &live, &mut self.automapper),
                        Tab::Orientation => tabs::orientation_tab(ui, &mut self.config.orientation),
                        Tab::Pressure => tabs::pressure_tab(ui, &mut self.config.pressure, &live),
                        Tab::Smoothing => tabs::smoothing_tab(ui, &mut self.config.smoothing),
                        Tab::Buttons => tabs::buttons_tab(ui, &mut self.config.buttons, &mut self.config.pen_buttons),
                        Tab::PressureTest => self.pressure_test.show(ui, &live, &self.config),
                    }
                });
            });

        if live.connected {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

use egui::{Color32, Stroke};
use ponzi_driver::usb::Tablet;
use ponzi_driver::virtual_device::{InputProcessor, VirtualKeys, VirtualPen};

fn usb_reader_thread(
    vid: u16, pid: u16,
    live: Arc<Mutex<LiveData>>,
    lock_signal: Arc<Mutex<Option<bool>>>,
    ctx: egui::Context,
    shared_cfg: Arc<Mutex<Config>>,
) {
    loop {
        log::debug!("looking for device {:04X}:{:04X}...", vid, pid);
        match Tablet::open(vid, pid) {
            Ok(mut tablet) => {
                log::debug!("device found, initializing...");
                if let Err(e) = tablet.init() {
                    log::error!("init: {}", e);
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.error_msg = format!("init: {}", e);
                    ctx.request_repaint();
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
                log::debug!("init OK, sending modeset...");
                if let Err(e) = tablet.set_full_mode() {
                    log::error!("modeset: {}", e);
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.error_msg = format!("modeset: {}", e);
                    ctx.request_repaint();
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
                log::debug!("modeset OK — creating virtual devices...");

                let cfg_snapshot = shared_cfg.lock().unwrap().clone();
                let driver_state = match DriverState::new(&cfg_snapshot) {
                    Ok(ds) => {
                        log::info!("virtual input devices created");
                        ds
                    }
                    Err(e) => {
                        log::error!("failed to create virtual devices: {}", e);
                        let mut l = live.lock().unwrap();
                        l.connected = true;
                        l.locked = false;
                        l.error_msg = format!("vdev: {}", e);
                        ctx.request_repaint();
                        let mut buf = vec![0u8; 64];
                        loop {
                            match tablet.read_input(&mut buf) {
                                Ok(_) => {
                                    let mut l = live.lock().unwrap();
                                    l.pen = PenData::decode(&buf);
                                }
                                Err(rusb::Error::Timeout) => {}
                                Err(_) => break,
                            }
                        }
                        continue;
                    }
                };

                let mut passthrough = true;

                {
                    let mut l = live.lock().unwrap();
                    l.connected = true;
                    l.locked = false;
                    l.error_msg.clear();
                }
                ctx.request_repaint();

                let mut pen = driver_state.pen;
                let mut keys = driver_state.keys;
                let mut processor = driver_state.processor;
                let mut buf = vec![0u8; 64];

                loop {
                    {
                        let mut sig = lock_signal.lock().unwrap();
                        if let Some(want_paused) = sig.take() {
                            passthrough = !want_paused;
                            log::info!("tablet {}", if passthrough { "active" } else { "paused" });
                            live.lock().unwrap().locked = want_paused;
                            ctx.request_repaint();
                        }
                    }

                    match tablet.read_input(&mut buf) {
                        Ok(_) => {
                            let data = PenData::decode(&buf);

                            if passthrough {
                                let cfg = shared_cfg.lock().unwrap();
                                processor.mapping = cfg.mapping.clone();
                                processor.orientation = cfg.orientation.clone();
                                processor.pressure = cfg.pressure.clone();
                                processor.smoothing = cfg.smoothing.clone();
                                drop(cfg);
                                processor.process(&data, &mut pen, &mut keys);
                            }

                            let mut l = live.lock().unwrap();
                            l.pen = data;
                            let threshold = 1510;
                            let range = 530;
                            if data.pressure_raw < threshold {
                                let norm = ((threshold - data.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
                                l.pressure_history.push(norm);
                                if l.pressure_history.len() > 500 {
                                    l.pressure_history.remove(0);
                                }
                            }
                        }
                        Err(rusb::Error::NoDevice) => {
                            log::info!("device disconnected");
                            break;
                        }
                        Err(rusb::Error::Timeout) => {}
                        Err(e) => {
                            log::error!("read error: {}", e);
                            break;
                        }
                    }
                }

                if live.lock().unwrap().connected {
                    log::debug!("connection lost");
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.locked = false;
                    l.error_msg = "Disconnected".into();
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                log::debug!("device not found: {}", e);
                let mut l = live.lock().unwrap();
                l.connected = false;
                l.locked = false;
                l.error_msg = format!("{}", e);
                ctx.request_repaint();
            }
        }
        thread::sleep(Duration::from_secs(3));
    }
}

struct DriverState {
    pen: VirtualPen,
    keys: VirtualKeys,
    processor: InputProcessor,
}

impl DriverState {
    fn new(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let pen = VirtualPen::new(&cfg.pressure, &cfg.mapping)?;
        let keys = VirtualKeys::new(&cfg.buttons)?;
        let processor = InputProcessor::new(cfg);
        Ok(Self { pen, keys, processor })
    }
}
