use eframe::egui;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod pressure_test;
mod tabs;

use ponzi_driver::config::Config;
use ponzi_driver::protocol::PenData;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([860.0, 680.0])
            .with_title("Ponzi — Tablet Configurator"),
        ..Default::default()
    };

    eframe::run_native(
        "Ponzi",
        options,
        Box::new(|cc| Ok(Box::new(PonziApp::new(cc)))),
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
    unlock_signal: Arc<Mutex<bool>>,
    active_tab: Tab,
    status_msg: String,
    pressure_test: pressure_test::PressureTest,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Status,
    Mapping,
    Orientation,
    Pressure,
    Smoothing,
    Buttons,
    PressureTest,
}

impl PonziApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let config_path = "config.toml".to_string();
        let config = Config::load(Path::new(&config_path)).unwrap_or_default();

        let live = Arc::new(Mutex::new(LiveData::default()));
        let unlock_signal = Arc::new(Mutex::new(false));
        let live_clone = Arc::clone(&live);
        let unlock_clone = Arc::clone(&unlock_signal);
        let ctx = cc.egui_ctx.clone();
        let vid = config.device.vendor_id;
        let pid = config.device.product_id;

        thread::spawn(move || usb_reader_thread(vid, pid, live_clone, unlock_clone, ctx));

        Self {
            config,
            config_path,
            live,
            unlock_signal,
            active_tab: Tab::Status,
            status_msg: String::new(),
            pressure_test: pressure_test::PressureTest::new(),
        }
    }

    fn save_config(&mut self) {
        match std::fs::write(&self.config_path, self.config.to_toml()) {
            Ok(()) => self.status_msg = "Config salva!".into(),
            Err(e) => self.status_msg = format!("Erro ao salvar: {}", e),
        }
    }
}

impl eframe::App for PonziApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let live = self.live.lock().unwrap().clone();

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Ponzi");
                ui.separator();
                if live.connected {
                    ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "⬤ Conectada");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(200, 80, 80), "⬤ Desconectada");
                    if !live.error_msg.is_empty() {
                        ui.label(egui::RichText::new(&live.error_msg).small().color(egui::Color32::from_rgb(200, 150, 100)));
                    }
                }
                if live.connected && live.locked {
                    if ui.button("Desbloquear").clicked() {
                        *self.unlock_signal.lock().unwrap() = true;
                    }
                    ui.label(egui::RichText::new("(Ponzi controlando)").small());
                } else if live.connected && !live.locked {
                    ui.label(egui::RichText::new("(Desbloqueada — outros apps podem usar)").small().color(egui::Color32::from_rgb(200, 200, 100)));
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Status, "Status");
                ui.selectable_value(&mut self.active_tab, Tab::Mapping, "Mapeamento");
                ui.selectable_value(&mut self.active_tab, Tab::Orientation, "Orientação");
                ui.selectable_value(&mut self.active_tab, Tab::Pressure, "Pressão");
                ui.selectable_value(&mut self.active_tab, Tab::Smoothing, "Suavização");
                ui.selectable_value(&mut self.active_tab, Tab::Buttons, "Botões");
                ui.selectable_value(&mut self.active_tab, Tab::PressureTest, "Teste");
            });
        });

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Salvar").clicked() {
                    self.save_config();
                }
                if ui.button("Restaurar Padrão").clicked() {
                    self.config = Config::default();
                    self.status_msg = "Restaurado ao padrão".into();
                }
                if !self.status_msg.is_empty() {
                    ui.separator();
                    ui.label(&self.status_msg);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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

fn usb_reader_thread(vid: u16, pid: u16, live: Arc<Mutex<LiveData>>, unlock: Arc<Mutex<bool>>, ctx: egui::Context) {
    loop {
        match ponzi_driver::usb::Tablet::open(vid, pid) {
            Ok(mut tablet) => {
                if let Err(e) = tablet.init() {
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.error_msg = format!("init: {}", e);
                    ctx.request_repaint();
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
                if let Err(e) = tablet.set_full_mode() {
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.error_msg = format!("modeset: {}", e);
                    ctx.request_repaint();
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }

                {
                    let mut l = live.lock().unwrap();
                    l.connected = true;
                    l.locked = true;
                    l.error_msg.clear();
                }
                ctx.request_repaint();

                let mut buf = vec![0u8; 64];
                loop {
                    // Check unlock signal
                    {
                        let mut sig = unlock.lock().unwrap();
                        if *sig {
                            *sig = false;
                            tablet.release();
                            let mut l = live.lock().unwrap();
                            l.locked = false;
                            ctx.request_repaint();
                            // Keep thread alive but idle — wait for reconnect
                            break;
                        }
                    }

                    match tablet.read_input(&mut buf) {
                        Ok(_) => {
                            let data = PenData::decode(&buf);
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
                        Err(rusb::Error::NoDevice) => break,
                        Err(rusb::Error::Timeout) => {}
                        Err(_) => break,
                    }
                }

                if live.lock().unwrap().locked {
                    let mut l = live.lock().unwrap();
                    l.connected = false;
                    l.locked = false;
                    l.error_msg = "Dispositivo desconectado".into();
                    ctx.request_repaint();
                }
            }
            Err(e) => {
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
