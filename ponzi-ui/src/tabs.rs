use egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

use crate::theme;
use crate::LiveData;
use ponzi_driver::config::{
    ButtonConfig, Config, InputMode, MappingConfig, OrientationConfig, PenButtonConfig, PressureConfig, SmoothingConfig,
};

enum CalibState {
    Idle,
    Step1 { hover_max: i32 },
    Step2 { hover_max: i32, light_min: i32 },
    Step3 { hover_max: i32, light_min: i32, hard_min: i32 },
}

fn progress_bar(ui: &mut egui::Ui, step: u8) {
    let (resp, painter) = ui.allocate_painter(Vec2::new(200.0, 6.0), egui::Sense::hover());
    let rect = resp.rect;
    painter.rect_filled(rect, 3, theme::BG_INPUT);
    let fill_w = (step as f32 / 3.0) * rect.width();
    let fill = egui::Rect::from_min_max(rect.left_top(), Pos2::new(rect.left() + fill_w, rect.bottom()));
    painter.rect_filled(fill, 3, theme::ACCENT);
}

fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(theme::section_heading(title));
    if !subtitle.is_empty() {
        ui.label(egui::RichText::new(subtitle).color(theme::TEXT_DIM).size(11.0));
    }
    ui.add_space(8.0);
}

// ── Status ───────────────────────────────────────────────────────────────

pub fn status_tab(ui: &mut egui::Ui, live: &LiveData, config: &Config) {
    page_heading(ui, "STATUS", "Real-time information from the drawing tablet");

    theme::section_frame().show(ui, |ui| {
        ui.columns(2, |cols| {
            let ui = &mut cols[0];
            ui.label(theme::label_dim("DEVICE"));
            ui.label(theme::label_mono(&format!("{:04X}:{:04X}", config.device.vendor_id, config.device.product_id)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("RESOLUTION"));
            ui.label(theme::label_mono(&format!("{}×{}", config.tablet.resolution_x, config.tablet.resolution_y)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("ROTATION"));
            ui.label(theme::label_mono(&format!("{}°", config.mapping.rotation)));

            let ui = &mut cols[1];
            ui.label(theme::label_dim("POSITION"));
            ui.label(theme::label_mono(&format!("X {:>5}  Y {:>5}", live.pen.x, live.pen.y)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("PRESSURE RAW"));
            ui.label(theme::label_mono(&format!("{:>5}", live.pen.pressure_raw)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("PEN"));
            let pen_str = match live.pen.pen_button {
                0 => "---",
                4 => "STYLUS",
                6 => "ERASER",
                _ => "???",
            };
            ui.label(theme::label_mono(pen_str));
        });
    });

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("PEN POSITION"));
            ui.add_space(4.0);
            let size = Vec2::new(180.0, 180.0);
            let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
            let rect = resp.rect;

            painter.rect_filled(rect, 2, theme::BG_INPUT);
            for i in 1..4 {
                let t = i as f32 / 4.0;
                let x = rect.left() + t * rect.width();
                let y = rect.top() + t * rect.height();
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(0.5, Color32::from_rgb(30, 35, 42)));
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(0.5, Color32::from_rgb(30, 35, 42)));
            }
            painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);

            if live.connected {
                let x = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
                let y = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
                let pos = Pos2::new(x, y);
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 40)));
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 40)));
                painter.circle_filled(pos, 4.0, theme::ACCENT);
                painter.circle_stroke(pos, 8.0, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 212, 170, 60)));
            }
        });

        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("PRESSURE"));
            ui.add_space(4.0);
            let norm = if live.connected && live.pen.pressure_raw < config.pressure.touch_threshold {
                ((config.pressure.touch_threshold - live.pen.pressure_raw) as f32
                    / config.pressure.pressure_range as f32).clamp(0.0, 1.0)
            } else { 0.0 };

            let bar_w = 32.0;
            let bar_h = 160.0;
            let (resp, painter) = ui.allocate_painter(Vec2::new(bar_w + 40.0, bar_h), egui::Sense::hover());
            let bar_rect = egui::Rect::from_min_size(resp.rect.left_top(), Vec2::new(bar_w, bar_h));
            painter.rect_filled(bar_rect, 2, theme::BG_INPUT);

            let fill_h = norm * bar_h;
            let fill_rect = egui::Rect::from_min_max(
                Pos2::new(bar_rect.left(), bar_rect.bottom() - fill_h),
                bar_rect.right_bottom(),
            );
            let fill_color = if norm > 0.8 { theme::ACCENT_GLOW } else if norm > 0.0 { theme::ACCENT } else { theme::BG_INPUT };
            painter.rect_filled(fill_rect, 2, fill_color);

            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let y = bar_rect.bottom() - t * bar_h;
                let tick_w = if i % 5 == 0 { 6.0 } else { 3.0 };
                painter.line_segment(
                    [Pos2::new(bar_rect.right() + 2.0, y), Pos2::new(bar_rect.right() + 2.0 + tick_w, y)],
                    Stroke::new(1.0, theme::TEXT_DIM));
            }
            painter.text(Pos2::new(bar_rect.right() + 14.0, bar_rect.top()), egui::Align2::LEFT_TOP, "100%", egui::FontId::monospace(9.0), theme::TEXT_DIM);
            painter.text(Pos2::new(bar_rect.right() + 14.0, bar_rect.bottom()), egui::Align2::LEFT_BOTTOM, "0%", egui::FontId::monospace(9.0), theme::TEXT_DIM);
            painter.rect_stroke(bar_rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);

            ui.add_space(4.0);
            ui.label(theme::label_mono(&format!("{:.0}%", norm * 100.0)));
        });
    });
}

// ── Mapping ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AutoMapper {
    active: bool,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    samples: u32,
}

pub fn mapping_tab(ui: &mut egui::Ui, m: &mut MappingConfig, live: &crate::LiveData, auto: &mut AutoMapper) {
    page_heading(ui, "MAPPING", "Active tablet area and screen destination");

    // Side-by-side: controls left, preview right
    ui.columns(2, |cols| {
        // ── Left column: sliders + options ──
        let ui = &mut cols[0];

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("TABLET AREA"));
            ui.add_space(4.0);
            egui::Grid::new("t_area").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("Left");   ui.add(egui::Slider::new(&mut m.tablet_left, 0..=4095)); ui.end_row();
                ui.label("Top");    ui.add(egui::Slider::new(&mut m.tablet_top, 0..=4095)); ui.end_row();
                ui.label("Right");  ui.add(egui::Slider::new(&mut m.tablet_right, 0..=4095)); ui.end_row();
                ui.label("Bottom"); ui.add(egui::Slider::new(&mut m.tablet_bottom, 0..=4095)); ui.end_row();
            });
        });

        ui.add_space(6.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("SCREEN AREA"));
            ui.add_space(4.0);
            egui::Grid::new("s_area").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label("Left");   ui.add(egui::Slider::new(&mut m.screen_left, 0..=4095)); ui.end_row();
                ui.label("Top");    ui.add(egui::Slider::new(&mut m.screen_top, 0..=4095)); ui.end_row();
                ui.label("Right");  ui.add(egui::Slider::new(&mut m.screen_right, 0..=4095)); ui.end_row();
                ui.label("Bottom"); ui.add(egui::Slider::new(&mut m.screen_bottom, 0..=4095)); ui.end_row();
            });
        });

        ui.add_space(6.0);

        theme::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::label_dim("MODE"));
                let abs_btn = egui::Button::new(
                    egui::RichText::new("Absolute").size(11.0)
                        .color(if m.mode == InputMode::Absolute { theme::BG_DEEP } else { theme::TEXT_SECONDARY })
                ).fill(if m.mode == InputMode::Absolute { theme::ACCENT } else { theme::BG_ELEVATED }).corner_radius(2);
                if ui.add(abs_btn).clicked() { m.mode = InputMode::Absolute; }

                let rel_btn = egui::Button::new(
                    egui::RichText::new("Relative (osu!)").size(11.0)
                        .color(if m.mode == InputMode::Relative { theme::BG_DEEP } else { theme::TEXT_SECONDARY })
                ).fill(if m.mode == InputMode::Relative { theme::ACCENT } else { theme::BG_ELEVATED }).corner_radius(2);
                if ui.add(rel_btn).clicked() { m.mode = InputMode::Relative; }
            });

            if m.mode == InputMode::Relative {
                ui.add_space(4.0);
                ui.add(egui::Slider::new(&mut m.sensitivity_x, 0.1..=10.0).text("Sensitivity X").logarithmic(true));
                ui.add(egui::Slider::new(&mut m.sensitivity_y, 0.1..=10.0).text("Sensitivity Y").logarithmic(true));
            }

            ui.add_space(4.0);
            ui.checkbox(&mut m.force_proportions, "Force proportions");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(theme::label_dim("ROTATION"));
                ui.radio_value(&mut m.rotation, 0, "0°");
                ui.radio_value(&mut m.rotation, 90, "90°");
                ui.radio_value(&mut m.rotation, 180, "180°");
                ui.radio_value(&mut m.rotation, 270, "270°");
            });
        });

        ui.add_space(6.0);

        // Automapper
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("AUTO-MAP"));
            ui.add_space(4.0);

            if !auto.active {
                ui.label(egui::RichText::new("Draw on the tablet to define the active area").color(theme::TEXT_DIM).size(10.5));
                let btn = egui::Button::new(
                    egui::RichText::new("▶ Start capture").size(12.0).color(theme::BG_DEEP)
                ).fill(theme::ACCENT).corner_radius(2);
                if ui.add(btn).clicked() {
                    auto.active = true;
                    auto.min_x = i32::MAX;
                    auto.min_y = i32::MAX;
                    auto.max_x = i32::MIN;
                    auto.max_y = i32::MIN;
                    auto.samples = 0;
                }
            } else {
                // Capture pen bounds
                if live.connected && (live.pen.x > 0 || live.pen.y > 0) {
                    auto.min_x = auto.min_x.min(live.pen.x);
                    auto.min_y = auto.min_y.min(live.pen.y);
                    auto.max_x = auto.max_x.max(live.pen.x);
                    auto.max_y = auto.max_y.max(live.pen.y);
                    auto.samples += 1;
                }

                ui.label(egui::RichText::new("⬤ Capturing — draw across the area you want to use")
                    .color(theme::RED).size(11.0));
                ui.label(theme::label_mono(&format!("Samples: {}  Area: ({},{}) → ({},{})",
                    auto.samples, auto.min_x, auto.min_y, auto.max_x, auto.max_y)));

                let btn = egui::Button::new(
                    egui::RichText::new("⏹ Apply").size(12.0).color(theme::BG_DEEP)
                ).fill(theme::ORANGE).corner_radius(2);
                if ui.add(btn).clicked() {
                    if auto.samples > 10 && auto.max_x > auto.min_x && auto.max_y > auto.min_y {
                        m.tablet_left = auto.min_x;
                        m.tablet_top = auto.min_y;
                        m.tablet_right = auto.max_x;
                        m.tablet_bottom = auto.max_y;
                        log::info!("auto-mapped area: ({},{}) → ({},{})", auto.min_x, auto.min_y, auto.max_x, auto.max_y);
                    }
                    auto.active = false;
                }
            }
        });

        // ── Right column: preview with live pen ──
        let ui = &mut cols[1];

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("PREVIEW"));
            ui.add_space(4.0);

            let avail = ui.available_width();
            let ps = Vec2::new(avail, avail);
            let (resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
            let rect = resp.rect;
            painter.rect_filled(rect, 2, theme::BG_INPUT);

            // Grid
            for i in 1..8 {
                let t = i as f32 / 8.0;
                painter.line_segment(
                    [Pos2::new(rect.left() + t * rect.width(), rect.top()), Pos2::new(rect.left() + t * rect.width(), rect.bottom())],
                    Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
                painter.line_segment(
                    [Pos2::new(rect.left(), rect.top() + t * rect.height()), Pos2::new(rect.right(), rect.top() + t * rect.height())],
                    Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
            }

            // Active area rectangle
            let tl = Pos2::new(
                rect.left() + (m.tablet_left as f32 / 4095.0) * rect.width(),
                rect.top() + (m.tablet_top as f32 / 4095.0) * rect.height(),
            );
            let br = Pos2::new(
                rect.left() + (m.tablet_right as f32 / 4095.0) * rect.width(),
                rect.top() + (m.tablet_bottom as f32 / 4095.0) * rect.height(),
            );
            let active = egui::Rect::from_min_max(tl, br);
            painter.rect_filled(active, 0, Color32::from_rgba_premultiplied(0, 212, 170, 20));
            painter.rect_stroke(active, 0, Stroke::new(1.5, theme::ACCENT), StrokeKind::Outside);

            for corner in [active.left_top(), active.right_top(), active.left_bottom(), active.right_bottom()] {
                painter.rect_filled(egui::Rect::from_center_size(corner, Vec2::splat(6.0)), 1, theme::ACCENT);
            }

            painter.text(
                Pos2::new(tl.x + 6.0, tl.y + 6.0), egui::Align2::LEFT_TOP,
                "Active area", egui::FontId::proportional(10.0), theme::ACCENT_DIM,
            );

            // Live pen position (raw on tablet)
            if live.connected && (live.pen.x > 0 || live.pen.y > 0) {
                let raw_x = live.pen.x as f32;
                let raw_y = live.pen.y as f32;
                let px = rect.left() + (raw_x / 4095.0) * rect.width();
                let py = rect.top() + (raw_y / 4095.0) * rect.height();
                let pos = Pos2::new(px, py);

                // Crosshair
                painter.line_segment([Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 50)));
                painter.line_segment([Pos2::new(rect.left(), py), Pos2::new(rect.right(), py)],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 50)));

                let color = if auto.active { theme::RED } else { theme::ACCENT };
                painter.circle_filled(pos, 4.0, color);
                painter.circle_stroke(pos, 8.0, Stroke::new(1.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80)));

                // Show mapped screen percentage
                let tl_x = m.tablet_left as f32;
                let tr_x = m.tablet_right as f32;
                let tl_y = m.tablet_top as f32;
                let tr_y = m.tablet_bottom as f32;
                let in_area = raw_x >= tl_x && raw_x <= tr_x && raw_y >= tl_y && raw_y <= tr_y;
                if in_area && tr_x > tl_x && tr_y > tl_y {
                    let screen_pct_x = ((raw_x - tl_x) / (tr_x - tl_x) * 100.0) as i32;
                    let screen_pct_y = ((raw_y - tl_y) / (tr_y - tl_y) * 100.0) as i32;
                    painter.text(
                        Pos2::new(px + 12.0, py - 4.0), egui::Align2::LEFT_BOTTOM,
                        format!("Screen {}%,{}%", screen_pct_x, screen_pct_y),
                        egui::FontId::monospace(9.0), theme::YELLOW,
                    );
                } else {
                    painter.text(
                        Pos2::new(px + 12.0, py - 4.0), egui::Align2::LEFT_BOTTOM,
                        "Outside", egui::FontId::monospace(9.0), theme::RED_DIM,
                    );
                }
            }

            // Automapper capture area preview
            if auto.active && auto.samples > 2 {
                let atl = Pos2::new(
                    rect.left() + (auto.min_x as f32 / 4095.0) * rect.width(),
                    rect.top() + (auto.min_y as f32 / 4095.0) * rect.height(),
                );
                let abr = Pos2::new(
                    rect.left() + (auto.max_x as f32 / 4095.0) * rect.width(),
                    rect.top() + (auto.max_y as f32 / 4095.0) * rect.height(),
                );
                let cap = egui::Rect::from_min_max(atl, abr);
                painter.rect_filled(cap, 0, Color32::from_rgba_premultiplied(255, 82, 82, 15));
                painter.rect_stroke(cap, 0, Stroke::new(1.5, theme::RED), StrokeKind::Outside);
                painter.text(
                    Pos2::new(atl.x + 4.0, abr.y - 4.0), egui::Align2::LEFT_BOTTOM,
                    "Capture", egui::FontId::proportional(10.0), theme::RED,
                );
            }

            painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
        });
    });
}

// ── Orientation ──────────────────────────────────────────────────────────

pub fn orientation_tab(ui: &mut egui::Ui, o: &mut OrientationConfig) {
    page_heading(ui, "ORIENTATION", "Mirroring and left-hand mode");

    theme::section_frame().show(ui, |ui| {
        ui.checkbox(&mut o.left_hand, "Left-hand mode");
        ui.label(egui::RichText::new("Mirrors the X axis for left-hand use").color(theme::TEXT_DIM).size(11.0));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut o.flip_x, "Flip X");
            ui.add_space(16.0);
            ui.checkbox(&mut o.flip_y, "Flip Y");
        });
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PREVIEW"));
        ui.add_space(4.0);
        let ps = Vec2::new(180.0, 180.0);
        let (resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
        let rect = resp.rect;
        painter.rect_filled(rect, 2, theme::BG_INPUT);
        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);

        let flip_x = o.flip_x ^ o.left_hand;
        let origin = Pos2::new(
            if flip_x { rect.right() - 24.0 } else { rect.left() + 24.0 },
            if o.flip_y { rect.bottom() - 24.0 } else { rect.top() + 24.0 },
        );
        let end_x = Pos2::new(if flip_x { rect.left() + 40.0 } else { rect.right() - 40.0 }, origin.y);
        let end_y = Pos2::new(origin.x, if o.flip_y { rect.top() + 40.0 } else { rect.bottom() - 40.0 });

        painter.arrow(origin, end_x - origin, Stroke::new(2.0, theme::RED));
        painter.arrow(origin, end_y - origin, Stroke::new(2.0, theme::GREEN));
        painter.text(end_x, egui::Align2::CENTER_BOTTOM, "X", egui::FontId::monospace(13.0), theme::RED);
        painter.text(end_y, egui::Align2::LEFT_CENTER, "Y", egui::FontId::monospace(13.0), theme::GREEN);
    });
}

// ── Pressure ─────────────────────────────────────────────────────────────

pub fn pressure_tab(ui: &mut egui::Ui, p: &mut PressureConfig, live: &crate::LiveData) {
    page_heading(ui, "PRESSURE", "Sensitivity and pen response curve");

    // Live readout
    if live.connected {
        theme::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::label_dim("RAW"));
                ui.label(theme::label_mono(&format!("{}", live.pen.pressure_raw)));
                ui.add_space(12.0);

                let touching = live.pen.pressure_raw < p.touch_threshold;
                let norm = if touching {
                    ((p.touch_threshold - live.pen.pressure_raw) as f32 / p.pressure_range as f32)
                        .clamp(0.0, 1.0).powf(p.gamma_value())
                } else { 0.0 };

                ui.label(theme::label_dim("OUT"));
                ui.label(theme::label_mono(&format!("{:.0}%", norm * 100.0)));
                ui.add_space(12.0);

                let (resp, painter) = ui.allocate_painter(Vec2::new(120.0, 14.0), egui::Sense::hover());
                let r = resp.rect;
                painter.rect_filled(r, 2, theme::BG_INPUT);
                let fill = egui::Rect::from_min_max(r.left_top(), Pos2::new(r.left() + norm * r.width(), r.bottom()));
                painter.rect_filled(fill, 2, theme::ACCENT);
            });
        });
        ui.add_space(6.0);
    }

    theme::section_frame().show(ui, |ui| {
        egui::Grid::new("p_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Touch threshold"); ui.add(egui::Slider::new(&mut p.touch_threshold, 500..=3000)); ui.end_row();
            ui.label("Range");           ui.add(egui::Slider::new(&mut p.pressure_range, 50..=2000)); ui.end_row();
            ui.label("Max reported");    ui.add(egui::Slider::new(&mut p.max_pressure, 1024..=65535)); ui.end_row();
            ui.label("Dead zone");       ui.add(egui::Slider::new(&mut p.min_pressure_threshold, 0..=5000)); ui.end_row();
        });
    });

    ui.add_space(6.0);

    // Guided calibration
    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("CALIBRATION"));
        ui.add_space(4.0);

        thread_local! {
            static CALIB: std::cell::RefCell<CalibState> = const { std::cell::RefCell::new(CalibState::Idle) };
        }

        CALIB.with(|c| {
            let mut state = c.borrow_mut();

            match &mut *state {
                CalibState::Idle => {
                    ui.label(egui::RichText::new("3-step guided calibration for optimal pressure response").color(theme::TEXT_DIM).size(10.5));
                    let btn = egui::Button::new(
                        egui::RichText::new("▶ Start calibration").size(12.0).color(theme::BG_DEEP)
                    ).fill(theme::ACCENT).corner_radius(2);
                    if ui.add(btn).clicked() {
                        *state = CalibState::Step1 { hover_max: 0 };
                        log::info!("calibration: step 1 — hover");
                    }
                }

                CalibState::Step1 { hover_max } => {
                    ui.colored_label(theme::YELLOW, "Step 1/3 — HOVER");
                    ui.label("Hold the pen close to the tablet WITHOUT touching. This detects the hover noise floor.");
                    ui.add_space(4.0);
                    progress_bar(ui, 1);

                    if live.connected && live.pen.pressure_raw > 0 {
                        *hover_max = (*hover_max).max(live.pen.pressure_raw);
                    }
                    ui.label(theme::label_mono(&format!("Hover max: {}", hover_max)));

                    let btn = egui::Button::new("Next →").fill(theme::ACCENT).corner_radius(2);
                    if ui.add(btn).clicked() {
                        let hm = *hover_max;
                        *state = CalibState::Step2 { hover_max: hm, light_min: i32::MAX };
                        log::info!("calibration: step 2 — light touch (hover_max={})", hm);
                    }
                }

                CalibState::Step2 { hover_max, light_min } => {
                    ui.colored_label(theme::ACCENT, "Step 2/3 — LIGHT TOUCH");
                    ui.label("Gently touch the tablet with the lightest pressure you'd use while drawing.");
                    ui.add_space(4.0);
                    progress_bar(ui, 2);

                    if live.connected && live.pen.pressure_raw > 0 && live.pen.pressure_raw < *hover_max {
                        *light_min = (*light_min).min(live.pen.pressure_raw);
                    }
                    ui.label(theme::label_mono(&format!("Light min: {}", if *light_min == i32::MAX { 0 } else { *light_min })));

                    let btn = egui::Button::new("Next →").fill(theme::ACCENT).corner_radius(2);
                    if ui.add(btn).clicked() {
                        let (hm, lm) = (*hover_max, *light_min);
                        *state = CalibState::Step3 { hover_max: hm, light_min: lm, hard_min: i32::MAX };
                        log::info!("calibration: step 3 — hard press (light_min={})", lm);
                    }
                }

                CalibState::Step3 { hover_max, light_min, hard_min } => {
                    ui.colored_label(theme::ORANGE, "Step 3/3 — HARD PRESS");
                    ui.label("Press as hard as you'd ever press while drawing. Give it your maximum.");
                    ui.add_space(4.0);
                    progress_bar(ui, 3);

                    if live.connected && live.pen.pressure_raw > 0 {
                        *hard_min = (*hard_min).min(live.pen.pressure_raw);
                    }
                    ui.label(theme::label_mono(&format!("Hard min: {}", if *hard_min == i32::MAX { 0 } else { *hard_min })));

                    let btn = egui::Button::new(
                        egui::RichText::new("✓ Apply").color(theme::BG_DEEP)
                    ).fill(theme::GREEN).corner_radius(2);
                    if ui.add(btn).clicked() {
                        let (hm, lm, hrd) = (*hover_max, *light_min, *hard_min);
                        if lm < hm && hrd < lm {
                            p.touch_threshold = lm + ((hm - lm) / 2);
                            p.pressure_range = lm - hrd + 20;
                            p.min_pressure_threshold = 0;
                            log::info!("calibrated: threshold={} range={} (hover={} light={} hard={})",
                                p.touch_threshold, p.pressure_range, hm, lm, hrd);
                        } else {
                            log::warn!("calibration values invalid: hover={} light={} hard={}", hm, lm, hrd);
                        }
                        *state = CalibState::Idle;
                    }
                }
            }

            if !matches!(*state, CalibState::Idle) {
                ui.add_space(4.0);
                let cancel = egui::Button::new(
                    egui::RichText::new("Cancel").color(theme::TEXT_DIM).size(11.0)
                ).fill(theme::BG_ELEVATED).corner_radius(2);
                if ui.add(cancel).clicked() {
                    *state = CalibState::Idle;
                }
            }
        });
    });

    ui.add_space(8.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PRESSURE CURVE"));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let presets = [("Linear", "linear"), ("Soft", "soft"), ("Firm", "firm"), ("Custom", "custom")];
            for (label, val) in presets {
                let selected = p.curve == val;
                let btn = egui::Button::new(
                    egui::RichText::new(label).size(11.0)
                        .color(if selected { theme::BG_DEEP } else { theme::TEXT_PRIMARY })
                ).fill(if selected { theme::ACCENT } else { theme::BG_ELEVATED })
                 .corner_radius(2);
                if ui.add(btn).clicked() {
                    p.curve = val.to_string();
                }
            }
        });

        if p.curve == "custom" {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut p.gamma, 0.1..=4.0).text("γ").logarithmic(true));
        }

        ui.add_space(8.0);

        let cw = ui.available_width().min(450.0);
        let cs = Vec2::new(cw, 200.0);
        let (resp, painter) = ui.allocate_painter(cs, egui::Sense::hover());
        let rect = resp.rect;
        painter.rect_filled(rect, 2, theme::BG_INPUT);

        for i in 1..4 {
            let t = i as f32 / 4.0;
            painter.line_segment([Pos2::new(rect.left() + t * rect.width(), rect.top()), Pos2::new(rect.left() + t * rect.width(), rect.bottom())],
                Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
            painter.line_segment([Pos2::new(rect.left(), rect.top() + t * rect.height()), Pos2::new(rect.right(), rect.top() + t * rect.height())],
                Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
        }

        painter.line_segment([rect.left_bottom(), rect.right_top()], Stroke::new(0.5, Color32::from_rgb(50, 55, 65)));

        let gamma = p.gamma_value();
        let steps = 120;
        let points: Vec<Pos2> = (0..=steps).map(|i| {
            let t = i as f32 / steps as f32;
            Pos2::new(rect.left() + t * rect.width(), rect.bottom() - t.powf(gamma) * rect.height())
        }).collect();

        for w in points.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(2.0, theme::ACCENT));
        }

        if p.min_pressure_threshold > 0 {
            let dz = p.min_pressure_threshold as f32 / p.max_pressure as f32;
            let y = rect.bottom() - dz * rect.height();
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 82, 82, 120)));
            painter.text(Pos2::new(rect.right() - 4.0, y - 2.0), egui::Align2::RIGHT_BOTTOM,
                "dead zone", egui::FontId::monospace(9.0), Color32::from_rgb(255, 82, 82));
        }

        painter.text(rect.left_bottom() + Vec2::new(4.0, -4.0), egui::Align2::LEFT_BOTTOM,
            "FORCE →", egui::FontId::monospace(9.0), theme::TEXT_DIM);
        painter.text(rect.left_top() + Vec2::new(4.0, 4.0), egui::Align2::LEFT_TOP,
            "↑ OUTPUT", egui::FontId::monospace(9.0), theme::TEXT_DIM);

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
    });
}

// ── Smoothing ────────────────────────────────────────────────────────────

pub fn smoothing_tab(ui: &mut egui::Ui, s: &mut SmoothingConfig) {
    page_heading(ui, "SMOOTHING", "Stroke filtering and stabilization");

    theme::section_frame().show(ui, |ui| {
        ui.checkbox(&mut s.enabled, "Stroke smoothing");
        if s.enabled {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut s.level, 1..=10).text("Level"));
        }
        ui.add_space(8.0);
        ui.checkbox(&mut s.anti_chatter, "Anti-chatter");
        if s.anti_chatter {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut s.anti_chatter_threshold, 1..=10).text("Frames"));
        }
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PREVIEW"));
        ui.add_space(4.0);

        let pw = ui.available_width().min(450.0);
        let ps = Vec2::new(pw, 100.0);
        let (resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
        let rect = resp.rect;
        painter.rect_filled(rect, 2, theme::BG_INPUT);

        let alpha = if s.enabled { 1.0 / s.level.max(1) as f32 } else { 1.0 };
        let mut raw_pts = Vec::new();
        let mut smooth_pts = Vec::new();
        let (mut sx, mut sy) = (0.0f32, 0.0f32);

        for i in 0..120 {
            let t = i as f32 / 119.0;
            let raw_y = (t * 12.0).sin() * 22.0 + (t * 37.0).sin() * 9.0;
            let px = rect.left() + t * rect.width();
            let py = rect.center().y + raw_y;
            raw_pts.push(Pos2::new(px, py));
            if i == 0 { sx = px; sy = py; }
            sx += (px - sx) * alpha;
            sy += (py - sy) * alpha;
            smooth_pts.push(Pos2::new(sx, sy));
        }

        for w in raw_pts.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 82, 82, 80)));
        }
        for w in smooth_pts.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(2.0, theme::ACCENT));
        }

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
    });
}

// ── Buttons ──────────────────────────────────────────────────────────────

pub fn buttons_tab(ui: &mut egui::Ui, buttons: &mut ButtonConfig, pen: &mut PenButtonConfig) {
    page_heading(ui, "BUTTONS", "Express key and pen button mapping");

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PEN"));
        ui.add_space(4.0);
        egui::Grid::new("pen_btns").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Tip touch");
            let tip_options = [
                ("none", "No click"),
                ("BTN_LEFT", "Left click"),
                ("BTN_RIGHT", "Right click"),
                ("BTN_MIDDLE", "Middle click"),
            ];
            egui::ComboBox::from_id_salt("tip_select")
                .selected_text(tip_options.iter().find(|(v, _)| *v == pen.tip).map(|(_, l)| *l).unwrap_or(&pen.tip))
                .width(160.0)
                .show_ui(ui, |ui| {
                    for (val, label) in &tip_options {
                        if ui.selectable_label(pen.tip == *val, *label).clicked() {
                            pen.tip = val.to_string();
                        }
                    }
                });
            ui.end_row();

            ui.label("Lower barrel");
            let barrel_options = [
                ("BTN_STYLUS", "Pen button"),
                ("BTN_LEFT", "Left click"),
                ("BTN_RIGHT", "Right click"),
                ("BTN_MIDDLE", "Middle click"),
                ("none", "Disabled"),
            ];
            egui::ComboBox::from_id_salt("stylus_select")
                .selected_text(barrel_options.iter().find(|(v, _)| *v == pen.stylus).map(|(_, l)| *l).unwrap_or(&pen.stylus))
                .width(160.0)
                .show_ui(ui, |ui| {
                    for (val, label) in &barrel_options {
                        if ui.selectable_label(pen.stylus == *val, *label).clicked() {
                            pen.stylus = val.to_string();
                        }
                    }
                });
            ui.end_row();

            ui.label("Upper barrel");
            let eraser_options = [
                ("BTN_STYLUS2", "Eraser button"),
                ("BTN_LEFT", "Left click"),
                ("BTN_RIGHT", "Right click"),
                ("BTN_MIDDLE", "Middle click"),
                ("none", "Disabled"),
            ];
            egui::ComboBox::from_id_salt("eraser_select")
                .selected_text(eraser_options.iter().find(|(v, _)| *v == pen.eraser).map(|(_, l)| *l).unwrap_or(&pen.eraser))
                .width(160.0)
                .show_ui(ui, |ui| {
                    for (val, label) in &eraser_options {
                        if ui.selectable_label(pen.eraser == *val, *label).clicked() {
                            pen.eraser = val.to_string();
                        }
                    }
                });
            ui.end_row();
        });
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("TABLET EXPRESS KEYS"));
        ui.label(egui::RichText::new("Keys separated by comma. Ex: LEFTCTRL, Z").color(theme::TEXT_DIM).size(10.0));
        ui.add_space(4.0);

        let labels = ["B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12"];
        let hints = ["Undo", "Redo", "Pen", "Eraser", "Select", "Pan", "Delete", "Duplicate", "Select All", "Zoom Fit", "Stroke Color", "Rectangle"];

        egui::Grid::new("express_btns").num_columns(3).spacing([8.0, 3.0]).show(ui, |ui| {
            for (i, (label, hint)) in labels.iter().zip(hints.iter()).enumerate() {
                let idx_color = if i < 6 { theme::ACCENT_DIM } else { theme::BLUE };
                ui.label(egui::RichText::new(*label).color(idx_color).family(egui::FontFamily::Monospace).size(11.0));

                let keys = buttons.keys_for_mut(i);
                let mut text = keys.join(", ");
                if ui.add_sized([220.0, 20.0], egui::TextEdit::singleline(&mut text)).changed() {
                    *keys = text.split(',').map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty()).collect();
                }

                ui.label(egui::RichText::new(format!("({})", hint)).color(theme::TEXT_DIM).size(9.5));
                ui.end_row();
            }
        });
    });
}
