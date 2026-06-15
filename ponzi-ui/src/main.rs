use eframe::egui;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod pressure_test;
mod tabs;
mod theme;

use ponzi_driver::config::Config;
use ponzi_driver::protocol::PenData;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("ponzi_ui=debug,ponzi_driver=debug,warn")
    ).init();

    ctrlc::set_handler(move || {
        log::info!("SIGINT received, exiting");
        std::process::exit(0);
    }).ok();

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
    config_path: String,
    live: Arc<Mutex<LiveData>>,
    lock_signal: Arc<Mutex<Option<bool>>>,
    active_tab: Tab,
    status_msg: String,
    status_timer: f64,
    pressure_test: pressure_test::PressureTest,
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

const CONFIG_PATHS: &[&str] = &["config.toml", "/etc/ponzi/config.toml"];

impl PonziApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let (config_path, config) = CONFIG_PATHS.iter()
            .find_map(|p| {
                log::info!("trying config: {}", p);
                Config::load(Path::new(p)).ok().map(|c| {
                    log::info!("loaded config from {}", p);
                    (p.to_string(), c)
                })
            })
            .unwrap_or_else(|| {
                log::info!("no config found, using defaults");
                ("config.toml".to_string(), Config::default())
            });

        let live = Arc::new(Mutex::new(LiveData::default()));
        let lock_signal: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
        let live_clone = Arc::clone(&live);
        let lock_clone = Arc::clone(&lock_signal);
        let ctx = cc.egui_ctx.clone();
        let vid = config.device.vendor_id;
        let pid = config.device.product_id;
        let cfg_clone = Config::default(); // driver uses defaults for virtual device

        log::info!("starting USB thread for {:04X}:{:04X}", vid, pid);
        thread::spawn(move || usb_reader_thread(vid, pid, live_clone, lock_clone, ctx, cfg_clone));

        Self {
            config,
            config_path,
            live,
            lock_signal,
            active_tab: Tab::Status,
            status_msg: String::new(),
            status_timer: 0.0,
            pressure_test: pressure_test::PressureTest::new(),
        }
    }

    fn save_config(&mut self) {
        match std::fs::write(&self.config_path, self.config.to_toml()) {
            Ok(()) => {
                log::info!("config saved to {}", self.config_path);
                self.status_msg = "Configuration saved".into();
            }
            Err(e) => {
                log::error!("saving config: {}", e);
                self.status_msg = format!("Error: {}", e);
            }
        }
        self.status_timer = 3.0;
    }
}

impl eframe::App for PonziApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let live = self.live.lock().unwrap().clone();

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
                    ui.label(egui::RichText::new("v0.1").color(theme::TEXT_DIM).size(10.0));
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    let (color, text) = if live.connected && live.locked {
                        (theme::GREEN, "Locked (driving)")
                    } else if live.connected {
                        (theme::YELLOW, "Connected (read-only)")
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

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

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

                    ui.horizontal(|ui| {
                        let save_btn = egui::Button::new(
                            egui::RichText::new("Save").color(theme::BG_DEEP).size(12.0)
                        ).fill(theme::ACCENT).corner_radius(3);
                        if ui.add_sized([76.0, 28.0], save_btn).clicked() {
                            self.save_config();
                        }

                        let reset_btn = egui::Button::new(
                            egui::RichText::new("Reset").color(theme::TEXT_SECONDARY).size(12.0)
                        ).fill(theme::BG_ELEVATED).corner_radius(3);
                        if ui.add_sized([60.0, 28.0], reset_btn).clicked() {
                            self.config = Config::default();
                            self.status_msg = "Reset".into();
                            self.status_timer = 2.0;
                        }
                    });

                    ui.add_space(6.0);

                    if live.connected && live.locked {
                        let btn = egui::Button::new(
                            egui::RichText::new("⏏ Unlock").color(theme::ORANGE).size(12.0)
                        ).fill(Color32::from_rgb(40, 32, 16)).corner_radius(3)
                         .stroke(Stroke::new(1.0, theme::ORANGE));
                        if ui.add_sized([ui.available_width(), 30.0], btn).clicked() {
                            log::info!("user requested UNLOCK");
                            *self.lock_signal.lock().unwrap() = Some(false);
                        }
                        ui.label(egui::RichText::new("Release for other apps").color(theme::TEXT_DIM).size(9.5));
                    } else if live.connected && !live.locked {
                        let btn = egui::Button::new(
                            egui::RichText::new("⬤ Lock & Drive").color(theme::GREEN).size(12.0)
                        ).fill(Color32::from_rgb(16, 36, 24)).corner_radius(3)
                         .stroke(Stroke::new(1.0, theme::GREEN));
                        if ui.add_sized([ui.available_width(), 30.0], btn).clicked() {
                            log::info!("user requested LOCK (create virtual devices)");
                            *self.lock_signal.lock().unwrap() = Some(true);
                        }
                        ui.label(egui::RichText::new("Tablet in read-only mode").color(theme::TEXT_DIM).size(9.5));
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
                        Tab::Mapping => tabs::mapping_tab(ui, &mut self.config.mapping),
                        Tab::Orientation => tabs::orientation_tab(ui, &mut self.config.orientation),
                        Tab::Pressure => tabs::pressure_tab(ui, &mut self.config.pressure),
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
    cfg: Config,
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

                let driver_state = match DriverState::new(&cfg) {
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
                        // Still read data for UI even without virtual devices
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

                // passthrough starts ON — virtual device always exists
                let mut passthrough = true;

                {
                    let mut l = live.lock().unwrap();
                    l.connected = true;
                    l.locked = true;
                    l.error_msg.clear();
                }
                ctx.request_repaint();

                let mut pen = driver_state.pen;
                let mut keys = driver_state.keys;
                let mut processor = driver_state.processor;
                let mut buf = vec![0u8; 64];

                loop {
                    // Check lock/unlock toggle (just flips passthrough, no reconnect)
                    {
                        let mut sig = lock_signal.lock().unwrap();
                        if let Some(want_locked) = sig.take() {
                            passthrough = want_locked;
                            log::info!("passthrough {}", if passthrough { "ON" } else { "OFF" });
                            live.lock().unwrap().locked = passthrough;
                            ctx.request_repaint();
                        }
                    }

                    match tablet.read_input(&mut buf) {
                        Ok(_) => {
                            let data = PenData::decode(&buf);

                            if passthrough {
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

                // Cleanup on disconnect
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
        log::debug!("creating pen: resolution {}x{}, max_pressure {}",
            cfg.mapping.screen_right - cfg.mapping.screen_left,
            cfg.mapping.screen_bottom - cfg.mapping.screen_top,
            cfg.pressure.max_pressure);
        let pen = VirtualPen::new(&cfg.pressure, &cfg.mapping)?;

        log::debug!("creating keyboard device...");
        let keys = VirtualKeys::new(&cfg.buttons)?;

        log::debug!("creating input processor...");
        let processor = InputProcessor::new(cfg);

        Ok(Self { pen, keys, processor })
    }
}
