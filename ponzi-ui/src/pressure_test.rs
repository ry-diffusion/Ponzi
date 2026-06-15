use egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

use crate::theme;
use crate::LiveData;
use ponzi_driver::config::Config;

pub struct PressureTest {
    strokes: Vec<Vec<StrokePoint>>,
    current_stroke: Vec<StrokePoint>,
    was_touching: bool,
}

#[derive(Clone)]
struct StrokePoint {
    pos: Pos2,
    pressure: f32,
}

impl PressureTest {
    pub fn new() -> Self {
        Self {
            strokes: Vec::new(),
            current_stroke: Vec::new(),
            was_touching: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, live: &LiveData, config: &Config) {
        ui.label(theme::section_heading("TESTE DE PRESSÃO"));
        ui.label(egui::RichText::new("Desenhe na mesa para testar pressão e posicionamento").color(theme::TEXT_DIM).size(11.0));
        ui.add_space(8.0);

        // Data readout bar
        theme::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let threshold = config.pressure.touch_threshold;
                let range = config.pressure.pressure_range;
                let gamma = config.pressure.gamma_value();
                let touching = live.pen.pressure_raw < threshold;

                let norm = if touching {
                    let linear = ((threshold - live.pen.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
                    linear.powf(gamma)
                } else {
                    0.0
                };

                readout(ui, "X", &format!("{:>5}", live.pen.x));
                ui.add_space(12.0);
                readout(ui, "Y", &format!("{:>5}", live.pen.y));
                ui.add_space(12.0);
                readout(ui, "RAW", &format!("{:>5}", live.pen.pressure_raw));
                ui.add_space(12.0);
                readout(ui, "OUT", &format!("{:>5.0}%", norm * 100.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let clear_btn = egui::Button::new(
                        egui::RichText::new("Limpar").color(theme::TEXT_SECONDARY).size(11.0)
                    ).fill(theme::BG_ELEVATED).corner_radius(2);
                    if ui.add(clear_btn).clicked() {
                        self.strokes.clear();
                        self.current_stroke.clear();
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Pressure history waveform
        let wave_h = 50.0;
        let (resp, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), wave_h),
            egui::Sense::hover(),
        );
        let rect = resp.rect;
        painter.rect_filled(rect, 2, theme::BG_INPUT);

        if !live.pressure_history.is_empty() {
            let len = live.pressure_history.len();
            let points: Vec<Pos2> = live.pressure_history.iter().enumerate().map(|(i, &p)| {
                let x = rect.left() + (i as f32 / 500.0) * rect.width();
                let y = rect.bottom() - p * rect.height();
                Pos2::new(x, y)
            }).collect();

            // Fill under curve
            for (i, w) in points.windows(2).enumerate() {
                let alpha = ((i as f32 / len as f32) * 30.0) as u8;
                painter.line_segment(
                    [Pos2::new(w[0].x, rect.bottom()), w[0]],
                    Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 212, 170, alpha)),
                );
            }

            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.5, theme::ACCENT));
            }
        }
        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(30, 35, 42)), StrokeKind::Outside);

        ui.add_space(8.0);

        // Canvas
        let canvas_h = ui.available_height().max(250.0);
        let canvas_size = Vec2::new(ui.available_width(), canvas_h);
        let (resp, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
        let rect = resp.rect;

        // Background with subtle grid
        painter.rect_filled(rect, 2, Color32::from_rgb(10, 12, 16));

        let grid_spacing = 40.0;
        let mut gx = rect.left();
        while gx < rect.right() {
            painter.line_segment(
                [Pos2::new(gx, rect.top()), Pos2::new(gx, rect.bottom())],
                Stroke::new(0.5, Color32::from_rgb(18, 22, 28)),
            );
            gx += grid_spacing;
        }
        let mut gy = rect.top();
        while gy < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), gy), Pos2::new(rect.right(), gy)],
                Stroke::new(0.5, Color32::from_rgb(18, 22, 28)),
            );
            gy += grid_spacing;
        }

        // Center crosshair
        let cx = rect.center().x;
        let cy = rect.center().y;
        painter.line_segment(
            [Pos2::new(cx, rect.top()), Pos2::new(cx, rect.bottom())],
            Stroke::new(0.5, Color32::from_rgb(25, 30, 36)),
        );
        painter.line_segment(
            [Pos2::new(rect.left(), cy), Pos2::new(rect.right(), cy)],
            Stroke::new(0.5, Color32::from_rgb(25, 30, 36)),
        );

        let threshold = config.pressure.touch_threshold;
        let range = config.pressure.pressure_range;
        let gamma = config.pressure.gamma_value();
        let touching = live.pen.pressure_raw < threshold;

        if touching && live.connected {
            let linear = ((threshold - live.pen.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
            let curved = linear.powf(gamma);

            let canvas_x = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
            let canvas_y = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
            let pos = Pos2::new(canvas_x, canvas_y);

            self.current_stroke.push(StrokePoint { pos, pressure: curved });

            if !self.was_touching {
                self.was_touching = true;
            }
        } else if self.was_touching {
            if !self.current_stroke.is_empty() {
                self.strokes.push(std::mem::take(&mut self.current_stroke));
            }
            self.was_touching = false;
        }

        // Render strokes
        for stroke in &self.strokes {
            draw_stroke(&painter, stroke);
        }
        draw_stroke(&painter, &self.current_stroke);

        // Pen cursor
        if live.connected && touching {
            let px = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
            let py = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
            let linear = ((threshold - live.pen.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
            let r = 4.0 + linear * 14.0;
            let pos = Pos2::new(px, py);

            painter.circle_stroke(pos, r, Stroke::new(1.5, theme::ACCENT));
            painter.circle_filled(pos, 2.0, theme::ACCENT_GLOW);
        } else if live.connected {
            // Hover indicator when not touching
            let px = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
            let py = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
            painter.circle_stroke(
                Pos2::new(px, py), 6.0,
                Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 60)),
            );
        }

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(30, 35, 42)), StrokeKind::Outside);
    }
}

fn readout(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).color(theme::TEXT_DIM).size(9.0));
        ui.label(egui::RichText::new(value).color(theme::ACCENT).family(egui::FontFamily::Monospace).size(13.0));
    });
}

fn draw_stroke(painter: &egui::Painter, points: &[StrokePoint]) {
    for w in points.windows(2) {
        let width = 1.0 + w[1].pressure * 10.0;
        let brightness = 0.4 + w[1].pressure * 0.6;
        let c = (brightness * 255.0) as u8;
        painter.line_segment(
            [w[0].pos, w[1].pos],
            Stroke::new(width, Color32::from_rgb(c, c, c)),
        );
    }
}
