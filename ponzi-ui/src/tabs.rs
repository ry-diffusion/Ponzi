use egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

use crate::theme;
use crate::LiveData;
use ponzi_driver::config::{
    ButtonConfig, Config, MappingConfig, OrientationConfig, PenButtonConfig, PressureConfig, SmoothingConfig,
};

fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(theme::section_heading(title));
    if !subtitle.is_empty() {
        ui.label(egui::RichText::new(subtitle).color(theme::TEXT_DIM).size(11.0));
    }
    ui.add_space(8.0);
}

// ── Status ───────────────────────────────────────────────────────────────

pub fn status_tab(ui: &mut egui::Ui, live: &LiveData, config: &Config) {
    page_heading(ui, "STATUS", "Informações em tempo real da mesa digitalizadora");

    theme::section_frame().show(ui, |ui| {
        ui.columns(2, |cols| {
            // Left column — device info
            let ui = &mut cols[0];
            ui.label(theme::label_dim("DISPOSITIVO"));
            ui.label(theme::label_mono(&format!("{:04X}:{:04X}", config.device.vendor_id, config.device.product_id)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("RESOLUÇÃO"));
            ui.label(theme::label_mono(&format!("{}×{}", config.tablet.resolution_x, config.tablet.resolution_y)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("ROTAÇÃO"));
            ui.label(theme::label_mono(&format!("{}°", config.mapping.rotation)));

            // Right column — live data
            let ui = &mut cols[1];
            ui.label(theme::label_dim("POSIÇÃO"));
            ui.label(theme::label_mono(&format!("X {:>5}  Y {:>5}", live.pen.x, live.pen.y)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("PRESSÃO RAW"));
            ui.label(theme::label_mono(&format!("{:>5}", live.pen.pressure_raw)));
            ui.add_space(8.0);

            ui.label(theme::label_dim("CANETA"));
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

    // Position radar + pressure bar side by side
    ui.horizontal(|ui| {
        // Pen position radar
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("POSIÇÃO DA CANETA"));
            ui.add_space(4.0);
            let size = Vec2::new(180.0, 180.0);
            let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
            let rect = resp.rect;

            painter.rect_filled(rect, 2, theme::BG_INPUT);

            // Grid lines
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

                // Crosshair
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 40)));
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(0, 212, 170, 40)));

                painter.circle_filled(pos, 4.0, theme::ACCENT);
                painter.circle_stroke(pos, 8.0, Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 212, 170, 60)));
            }
        });

        ui.add_space(8.0);

        // Pressure meter
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("PRESSÃO"));
            ui.add_space(4.0);

            let norm = if live.connected && live.pen.pressure_raw < config.pressure.touch_threshold {
                ((config.pressure.touch_threshold - live.pen.pressure_raw) as f32
                    / config.pressure.pressure_range as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            // Vertical bar meter
            let bar_w = 32.0;
            let bar_h = 160.0;
            let (resp, painter) = ui.allocate_painter(Vec2::new(bar_w + 40.0, bar_h), egui::Sense::hover());
            let bar_rect = egui::Rect::from_min_size(
                Pos2::new(resp.rect.left(), resp.rect.top()),
                Vec2::new(bar_w, bar_h),
            );

            painter.rect_filled(bar_rect, 2, theme::BG_INPUT);

            // Fill from bottom
            let fill_h = norm * bar_h;
            let fill_rect = egui::Rect::from_min_max(
                Pos2::new(bar_rect.left(), bar_rect.bottom() - fill_h),
                bar_rect.right_bottom(),
            );
            let fill_color = if norm > 0.8 {
                theme::ACCENT_GLOW
            } else if norm > 0.0 {
                theme::ACCENT
            } else {
                theme::BG_INPUT
            };
            painter.rect_filled(fill_rect, 2, fill_color);

            // Tick marks
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let y = bar_rect.bottom() - t * bar_h;
                let tick_w = if i % 5 == 0 { 6.0 } else { 3.0 };
                painter.line_segment(
                    [Pos2::new(bar_rect.right() + 2.0, y), Pos2::new(bar_rect.right() + 2.0 + tick_w, y)],
                    Stroke::new(1.0, theme::TEXT_DIM),
                );
            }

            painter.text(
                Pos2::new(bar_rect.right() + 14.0, bar_rect.top()),
                egui::Align2::LEFT_TOP, "100%",
                egui::FontId::monospace(9.0), theme::TEXT_DIM,
            );
            painter.text(
                Pos2::new(bar_rect.right() + 14.0, bar_rect.bottom()),
                egui::Align2::LEFT_BOTTOM, "0%",
                egui::FontId::monospace(9.0), theme::TEXT_DIM,
            );

            painter.rect_stroke(bar_rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);

            ui.add_space(4.0);
            ui.label(theme::label_mono(&format!("{:.0}%", norm * 100.0)));
        });
    });
}

// ── Mapping ──────────────────────────────────────────────────────────────

pub fn mapping_tab(ui: &mut egui::Ui, m: &mut MappingConfig) {
    page_heading(ui, "MAPEAMENTO", "Área ativa da mesa e destino na tela");

    ui.horizontal(|ui| {
        // Tablet area
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("ÁREA DA MESA"));
            ui.add_space(4.0);
            egui::Grid::new("t_area").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label(theme::label_dim("L")); ui.add(egui::Slider::new(&mut m.tablet_left, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("T")); ui.add(egui::Slider::new(&mut m.tablet_top, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("R")); ui.add(egui::Slider::new(&mut m.tablet_right, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("B")); ui.add(egui::Slider::new(&mut m.tablet_bottom, 0..=4095)); ui.end_row();
            });
        });

        ui.add_space(8.0);

        // Screen area
        theme::section_frame().show(ui, |ui| {
            ui.label(theme::label_dim("ÁREA DA TELA"));
            ui.add_space(4.0);
            egui::Grid::new("s_area").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                ui.label(theme::label_dim("L")); ui.add(egui::Slider::new(&mut m.screen_left, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("T")); ui.add(egui::Slider::new(&mut m.screen_top, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("R")); ui.add(egui::Slider::new(&mut m.screen_right, 0..=4095)); ui.end_row();
                ui.label(theme::label_dim("B")); ui.add(egui::Slider::new(&mut m.screen_bottom, 0..=4095)); ui.end_row();
            });
        });
    });

    ui.add_space(8.0);

    theme::section_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut m.force_proportions, "");
            ui.label("Forçar proporções");
            ui.add_space(24.0);

            ui.label(theme::label_dim("ROTAÇÃO"));
            ui.radio_value(&mut m.rotation, 0, "0°");
            ui.radio_value(&mut m.rotation, 90, "90°");
            ui.radio_value(&mut m.rotation, 180, "180°");
            ui.radio_value(&mut m.rotation, 270, "270°");
        });
    });

    ui.add_space(12.0);

    // Preview
    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PREVIEW"));
        ui.add_space(4.0);
        let ps = Vec2::new(ui.available_width().min(400.0), 220.0);
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

        // Active area
        let tl = Pos2::new(
            rect.left() + (m.tablet_left as f32 / 4095.0) * rect.width(),
            rect.top() + (m.tablet_top as f32 / 4095.0) * rect.height(),
        );
        let br = Pos2::new(
            rect.left() + (m.tablet_right as f32 / 4095.0) * rect.width(),
            rect.top() + (m.tablet_bottom as f32 / 4095.0) * rect.height(),
        );
        let active = egui::Rect::from_min_max(tl, br);
        painter.rect_filled(active, 0, Color32::from_rgba_premultiplied(0, 212, 170, 15));
        painter.rect_stroke(active, 0, Stroke::new(1.5, theme::ACCENT), StrokeKind::Outside);

        // Corner handles
        for corner in [active.left_top(), active.right_top(), active.left_bottom(), active.right_bottom()] {
            painter.rect_filled(
                egui::Rect::from_center_size(corner, Vec2::splat(6.0)),
                1, theme::ACCENT,
            );
        }

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
    });
}

// ── Orientation ──────────────────────────────────────────────────────────

pub fn orientation_tab(ui: &mut egui::Ui, o: &mut OrientationConfig) {
    page_heading(ui, "ORIENTAÇÃO", "Espelhamento e modo canhoto");

    theme::section_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut o.left_hand, "");
            ui.label("Modo canhoto");
        });
        ui.label(egui::RichText::new("Espelha o eixo X para uso com mão esquerda").color(theme::TEXT_DIM).size(11.0));

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut o.flip_x, "");
            ui.label("Espelhar X");
            ui.add_space(16.0);
            ui.checkbox(&mut o.flip_y, "");
            ui.label("Espelhar Y");
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
        let end_x = Pos2::new(
            if flip_x { rect.left() + 40.0 } else { rect.right() - 40.0 },
            origin.y,
        );
        let end_y = Pos2::new(
            origin.x,
            if o.flip_y { rect.top() + 40.0 } else { rect.bottom() - 40.0 },
        );

        painter.arrow(origin, end_x - origin, Stroke::new(2.0, theme::RED));
        painter.arrow(origin, end_y - origin, Stroke::new(2.0, theme::GREEN));

        painter.text(end_x, egui::Align2::CENTER_BOTTOM, "X", egui::FontId::monospace(13.0), theme::RED);
        painter.text(end_y, egui::Align2::LEFT_CENTER, "Y", egui::FontId::monospace(13.0), theme::GREEN);
    });
}

// ── Pressure ─────────────────────────────────────────────────────────────

pub fn pressure_tab(ui: &mut egui::Ui, p: &mut PressureConfig) {
    page_heading(ui, "PRESSÃO", "Sensibilidade e curva de resposta da caneta");

    theme::section_frame().show(ui, |ui| {
        egui::Grid::new("p_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(theme::label_dim("LIMIAR DE TOQUE"));
            ui.add(egui::Slider::new(&mut p.touch_threshold, 500..=3000));
            ui.end_row();

            ui.label(theme::label_dim("FAIXA"));
            ui.add(egui::Slider::new(&mut p.pressure_range, 50..=2000));
            ui.end_row();

            ui.label(theme::label_dim("MÁX REPORTADA"));
            ui.add(egui::Slider::new(&mut p.max_pressure, 1024..=65535));
            ui.end_row();

            ui.label(theme::label_dim("DEAD ZONE"));
            ui.add(egui::Slider::new(&mut p.min_pressure_threshold, 0..=5000));
            ui.end_row();
        });
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("CURVA DE PRESSÃO"));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let presets = [("Linear", "linear"), ("Soft", "soft"), ("Firm", "firm"), ("Custom", "custom")];
            for (label, val) in presets {
                let selected = p.curve == val;
                let btn = egui::Button::new(
                    egui::RichText::new(label).size(11.0)
                        .color(if selected { theme::BG_DEEP } else { theme::TEXT_SECONDARY })
                ).fill(if selected { theme::ACCENT } else { theme::BG_ELEVATED })
                 .corner_radius(2);
                if ui.add_sized([64.0, 24.0], btn).clicked() {
                    p.curve = val.to_string();
                }
            }
        });

        if p.curve == "custom" {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut p.gamma, 0.1..=4.0).text("γ").logarithmic(true));
        }

        ui.add_space(8.0);

        // Curve plot
        let cs = Vec2::new(ui.available_width().min(400.0), 200.0);
        let (resp, painter) = ui.allocate_painter(cs, egui::Sense::hover());
        let rect = resp.rect;

        painter.rect_filled(rect, 2, theme::BG_INPUT);

        // Grid
        for i in 1..4 {
            let t = i as f32 / 4.0;
            let x = rect.left() + t * rect.width();
            let y = rect.top() + t * rect.height();
            painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
            painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5, Color32::from_rgb(25, 30, 36)));
        }

        // Linear reference line (diagonal)
        painter.line_segment(
            [rect.left_bottom(), rect.right_top()],
            Stroke::new(0.5, Color32::from_rgb(50, 55, 65)),
        );

        // Actual curve
        let gamma = p.gamma_value();
        let steps = 120;
        let points: Vec<Pos2> = (0..=steps).map(|i| {
            let t = i as f32 / steps as f32;
            let out = t.powf(gamma);
            Pos2::new(rect.left() + t * rect.width(), rect.bottom() - out * rect.height())
        }).collect();

        for w in points.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(2.0, theme::ACCENT));
        }

        // Dead zone
        if p.min_pressure_threshold > 0 {
            let dz = p.min_pressure_threshold as f32 / p.max_pressure as f32;
            let y = rect.bottom() - dz * rect.height();
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 82, 82, 120)),
            );
        }

        // Axis labels
        painter.text(rect.left_bottom() + Vec2::new(4.0, -4.0), egui::Align2::LEFT_BOTTOM,
            "FORÇA →", egui::FontId::monospace(9.0), theme::TEXT_DIM);
        painter.text(rect.left_top() + Vec2::new(4.0, 4.0), egui::Align2::LEFT_TOP,
            "↑ OUTPUT", egui::FontId::monospace(9.0), theme::TEXT_DIM);

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
    });
}

// ── Smoothing ────────────────────────────────────────────────────────────

pub fn smoothing_tab(ui: &mut egui::Ui, s: &mut SmoothingConfig) {
    page_heading(ui, "SUAVIZAÇÃO", "Filtragem de traço e estabilização");

    theme::section_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.enabled, "");
            ui.label("Suavização de traço");
        });
        if s.enabled {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut s.level, 1..=10).text("Nível"));
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.anti_chatter, "");
            ui.label("Anti-chatter");
        });
        if s.anti_chatter {
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut s.anti_chatter_threshold, 1..=10).text("Frames"));
        }
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("PREVIEW"));
        ui.add_space(4.0);

        let ps = Vec2::new(ui.available_width().min(400.0), 100.0);
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
            painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 82, 82, 60)));
        }
        for w in smooth_pts.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(2.0, theme::ACCENT));
        }

        painter.rect_stroke(rect, 2, Stroke::new(1.0, Color32::from_rgb(40, 46, 55)), StrokeKind::Outside);
    });
}

// ── Buttons ──────────────────────────────────────────────────────────────

pub fn buttons_tab(ui: &mut egui::Ui, buttons: &mut ButtonConfig, pen: &mut PenButtonConfig) {
    page_heading(ui, "BOTÕES", "Mapeamento dos botões express e caneta");

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("CANETA"));
        ui.add_space(4.0);
        egui::Grid::new("pen_btns").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Inferior");
            ui.add_sized([180.0, 20.0], egui::TextEdit::singleline(&mut pen.stylus));
            ui.end_row();
            ui.label("Superior");
            ui.add_sized([180.0, 20.0], egui::TextEdit::singleline(&mut pen.eraser));
            ui.end_row();
        });
    });

    ui.add_space(12.0);

    theme::section_frame().show(ui, |ui| {
        ui.label(theme::label_dim("BOTÕES EXPRESS"));
        ui.add_space(4.0);

        let labels = ["B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12"];
        let hints = ["E", "B", "Ctrl+-", "Ctrl++", "[", "]", "ScrollUp", "Tab", "ScrollDown", "Space", "Ctrl", "Alt"];

        egui::Grid::new("express_btns").num_columns(3).spacing([8.0, 3.0]).show(ui, |ui| {
            for (i, (label, hint)) in labels.iter().zip(hints.iter()).enumerate() {
                let idx_color = if i < 6 { theme::ACCENT_DIM } else { theme::BLUE };
                ui.label(egui::RichText::new(*label).color(idx_color).family(egui::FontFamily::Monospace).size(11.0));

                let keys = buttons.keys_for_mut(i);
                let mut text = keys.join(", ");
                if ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut text)).changed() {
                    *keys = text.split(',').map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty()).collect();
                }

                ui.label(egui::RichText::new(*hint).color(theme::TEXT_DIM).size(9.5));
                ui.end_row();
            }
        });
    });
}
