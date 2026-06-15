use egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

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
        ui.horizontal(|ui| {
            ui.heading("Teste de Pressão");
            if ui.button("Limpar").clicked() {
                self.strokes.clear();
                self.current_stroke.clear();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!("X: {}", live.pen.x));
            ui.label(format!("Y: {}", live.pen.y));
            ui.label(format!("Pressão raw: {}", live.pen.pressure_raw));

            let threshold = config.pressure.touch_threshold;
            let range = config.pressure.pressure_range;
            let gamma = config.pressure.gamma_value();
            let touching = live.pen.pressure_raw < threshold;

            if touching {
                let linear = ((threshold - live.pen.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
                let curved = linear.powf(gamma);
                ui.label(format!("Pressão: {:.0}%", curved * 100.0));
            } else {
                ui.label("Pressão: 0%");
            }
        });

        ui.separator();

        // Pressure history graph
        ui.label("Histórico de pressão:");
        let graph_height = 60.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), graph_height),
            egui::Sense::hover(),
        );
        let rect = response.rect;

        painter.rect_filled(rect, 4.0, Color32::from_gray(30));

        if !live.pressure_history.is_empty() {
            let points: Vec<Pos2> = live.pressure_history.iter().enumerate().map(|(i, &p)| {
                let x = rect.left() + (i as f32 / 500.0) * rect.width();
                let y = rect.bottom() - p * rect.height();
                Pos2::new(x, y)
            }).collect();

            for w in points.windows(2) {
                painter.line_segment(
                    [w[0], w[1]],
                    Stroke::new(2.0, Color32::from_rgb(100, 200, 255)),
                );
            }
        }

        ui.separator();

        // Drawing canvas
        ui.label("Área de desenho (mova a caneta na mesa):");
        let canvas_size = Vec2::new(ui.available_width(), ui.available_height().max(200.0));
        let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
        let rect = response.rect;

        painter.rect_filled(rect, 4.0, Color32::from_gray(20));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);

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

        // Draw all completed strokes
        for stroke in &self.strokes {
            draw_stroke(&painter, stroke);
        }
        // Draw current stroke
        draw_stroke(&painter, &self.current_stroke);

        // Draw cursor
        if live.connected && touching {
            let cx = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
            let cy = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
            let linear = ((threshold - live.pen.pressure_raw) as f32 / range as f32).clamp(0.0, 1.0);
            let radius = 3.0 + linear * 12.0;
            painter.circle_stroke(
                Pos2::new(cx, cy),
                radius,
                Stroke::new(1.5, Color32::from_rgb(255, 100, 100)),
            );
        }
    }
}

fn draw_stroke(painter: &egui::Painter, points: &[StrokePoint]) {
    for w in points.windows(2) {
        let width = 1.0 + w[1].pressure * 8.0;
        let alpha = (60.0 + w[1].pressure * 195.0) as u8;
        painter.line_segment(
            [w[0].pos, w[1].pos],
            Stroke::new(width, Color32::from_rgba_premultiplied(255, 255, 255, alpha)),
        );
    }
}
