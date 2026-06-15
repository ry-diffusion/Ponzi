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
    env_logger::init();

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
    unlock_signal: Arc<Mutex<bool>>,
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
            Self::Mapping => "Mapeamento",
            Self::Orientation => "Orientação",
            Self::Pressure => "Pressão",
            Self::Smoothing => "Suavização",
            Self::Buttons => "Botões",
            Self::PressureTest => "Teste",
        }
    }

    const ALL: &[Tab] = &[
        Self::Status, Self::Mapping, Self::Orientation,
        Self::Pressure, Self::Smoothing, Self::Buttons, Self::PressureTest,
    ];
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
            status_timer: 0.0,
            pressure_test: pressure_test::PressureTest::new(),
        }
    }

    fn save_config(&mut self) {
        match std::fs::write(&self.config_path, self.config.to_toml()) {
            Ok(()) => self.status_msg = "Configuração salva".into(),
            Err(e) => self.status_msg = format!("Erro: {}", e),
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
                // Logo / title
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("PONZI").color(theme::ACCENT).size(18.0).strong());
                    ui.label(egui::RichText::new("v0.1").color(theme::TEXT_DIM).size(10.0));
                });
                ui.add_space(2.0);

                // Connection status LED
                ui.horizontal(|ui| {
                    let (color, text) = if live.connected && live.locked {
                        (theme::GREEN, "Conectada")
                    } else if live.connected {
                        (theme::YELLOW, "Desbloqueada")
                    } else {
                        (theme::RED_DIM, "Desconectada")
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
                            ui.painter().rect_filled(rect, 3, Color32::from_rgba_premultiplied(0, 212, 170, 15));
                            let rail = egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height()));
                            ui.painter().rect_filled(rail, 1, theme::ACCENT);
                        } else if hovered {
                            ui.painter().rect_filled(rect, 3, Color32::from_rgba_premultiplied(255, 255, 255, 6));
                        }

                        let text_color = if selected { theme::ACCENT } else if hovered { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY };

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

                // Bottom section — lock/unlock + save
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    if !self.status_msg.is_empty() {
                        ui.label(egui::RichText::new(&self.status_msg).color(theme::ACCENT).size(10.0));
                        ui.add_space(4.0);
                    }

                    ui.horizontal(|ui| {
                        let save_btn = egui::Button::new(
                            egui::RichText::new("Salvar").color(theme::BG_DEEP).size(12.0)
                        ).fill(theme::ACCENT).corner_radius(3);
                        if ui.add_sized([76.0, 28.0], save_btn).clicked() {
                            self.save_config();
                        }

                        let reset_btn = egui::Button::new(
                            egui::RichText::new("Reset").color(theme::TEXT_SECONDARY).size(12.0)
                        ).fill(theme::BG_ELEVATED).corner_radius(3);
                        if ui.add_sized([60.0, 28.0], reset_btn).clicked() {
                            self.config = Config::default();
                            self.status_msg = "Restaurado".into();
                            self.status_timer = 2.0;
                        }
                    });

                    ui.add_space(6.0);

                    if live.connected && live.locked {
                        let unlock_btn = egui::Button::new(
                            egui::RichText::new("⏏ Desbloquear").color(theme::ORANGE).size(12.0)
                        ).fill(Color32::from_rgb(40, 32, 16)).corner_radius(3)
                         .stroke(Stroke::new(1.0, theme::ORANGE));
                        if ui.add_sized([ui.available_width(), 30.0], unlock_btn).clicked() {
                            *self.unlock_signal.lock().unwrap() = true;
                        }
                        ui.label(egui::RichText::new("Libera para outros apps").color(theme::TEXT_DIM).size(9.5));
                    } else if live.connected && !live.locked {
                        let lock_frame = egui::Frame::default()
                            .fill(Color32::from_rgb(30, 36, 20))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(60, 70, 40)));
                        lock_frame.show(ui, |ui| {
                            ui.label(egui::RichText::new("Mesa desbloqueada").color(theme::YELLOW).size(11.0));
                            ui.label(egui::RichText::new("Outros apps podem usar").color(theme::TEXT_DIM).size(9.5));
                        });
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
                    {
                        let mut sig = unlock.lock().unwrap();
                        if *sig {
                            *sig = false;
                            tablet.release();
                            let mut l = live.lock().unwrap();
                            l.locked = false;
                            ctx.request_repaint();
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
                    l.error_msg = "Desconectado".into();
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
