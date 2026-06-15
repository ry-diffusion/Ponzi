use egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

use crate::LiveData;
use ponzi_driver::config::{
    ButtonConfig, Config, MappingConfig, OrientationConfig, PenButtonConfig, PressureConfig, SmoothingConfig,
};

// ── Status ───────────────────────────────────────────────────────────────

pub fn status_tab(ui: &mut egui::Ui, live: &LiveData, config: &Config) {
    ui.heading("Status da Mesa");
    ui.separator();

    egui::Grid::new("status_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
        ui.label("Dispositivo:");
        ui.label(format!("VID {:04X} / PID {:04X}", config.device.vendor_id, config.device.product_id));
        ui.end_row();

        ui.label("Conexão:");
        if live.connected {
            ui.colored_label(Color32::from_rgb(80, 200, 80), "Conectada");
        } else {
            ui.colored_label(Color32::from_rgb(200, 80, 80), "Desconectada");
        }
        ui.end_row();

        if !live.error_msg.is_empty() {
            ui.label("Erro:");
            ui.colored_label(Color32::from_rgb(255, 180, 80), &live.error_msg);
            ui.end_row();
        }

        ui.label("Posição:");
        ui.label(format!("X: {}  Y: {}", live.pen.x, live.pen.y));
        ui.end_row();

        ui.label("Pressão raw:");
        ui.label(format!("{}", live.pen.pressure_raw));
        ui.end_row();

        ui.label("Botão caneta:");
        let pen_btn = match live.pen.pen_button {
            4 => "Stylus (inferior)".to_string(),
            6 => "Eraser (superior)".to_string(),
            0 => "Nenhum".to_string(),
            v => format!("0x{:02X}", v),
        };
        ui.label(pen_btn);
        ui.end_row();

        ui.label("Botões mesa:");
        ui.label(format!("0x{:04X}", live.pen.tablet_buttons));
        ui.end_row();

        ui.label("Resolução:");
        ui.label(format!("{}x{}", config.tablet.resolution_x, config.tablet.resolution_y));
        ui.end_row();

        ui.label("Rotação:");
        ui.label(format!("{}°", config.mapping.rotation));
        ui.end_row();
    });

    ui.separator();
    ui.label("Posição da caneta:");
    let size = Vec2::new(200.0, 200.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;

    painter.rect_filled(rect, 4.0, Color32::from_gray(30));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);

    if live.connected {
        let x = rect.left() + (live.pen.x as f32 / 4095.0) * rect.width();
        let y = rect.top() + (live.pen.y as f32 / 4095.0) * rect.height();
        painter.circle_filled(Pos2::new(x, y), 4.0, Color32::from_rgb(100, 200, 255));
    }

    // Pressure bar
    ui.add_space(8.0);
    ui.label("Pressão:");
    let bar_h = 20.0;
    let (resp, painter) = ui.allocate_painter(Vec2::new(ui.available_width().min(400.0), bar_h), egui::Sense::hover());
    let r = resp.rect;
    painter.rect_filled(r, 4.0, Color32::from_gray(30));

    if live.connected && live.pen.pressure_raw < config.pressure.touch_threshold {
        let norm = ((config.pressure.touch_threshold - live.pen.pressure_raw) as f32
            / config.pressure.pressure_range as f32).clamp(0.0, 1.0);
        let fill_r = egui::Rect::from_min_max(
            r.left_top(),
            Pos2::new(r.left() + norm * r.width(), r.bottom()),
        );
        painter.rect_filled(fill_r, 4.0, Color32::from_rgb(100, 200, 120));
    }
}

// ── Mapping ──────────────────────────────────────────────────────────────

pub fn mapping_tab(ui: &mut egui::Ui, m: &mut MappingConfig) {
    ui.heading("Mapeamento de Área");
    ui.label("Configure qual parte da mesa é usada e como mapeia para a tela.");
    ui.separator();

    ui.heading("Área ativa da mesa");
    ui.label("Coordenadas do hardware (0–4095). Restrinja para usar só parte da mesa.");
    egui::Grid::new("tablet_area").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Esquerda:"); ui.add(egui::Slider::new(&mut m.tablet_left, 0..=4095)); ui.end_row();
        ui.label("Topo:");     ui.add(egui::Slider::new(&mut m.tablet_top, 0..=4095)); ui.end_row();
        ui.label("Direita:");  ui.add(egui::Slider::new(&mut m.tablet_right, 0..=4095)); ui.end_row();
        ui.label("Base:");     ui.add(egui::Slider::new(&mut m.tablet_bottom, 0..=4095)); ui.end_row();
    });

    ui.add_space(12.0);
    ui.heading("Área de destino na tela");
    ui.label("Para onde a área da mesa mapeia (0–4095).");
    egui::Grid::new("screen_area").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Esquerda:"); ui.add(egui::Slider::new(&mut m.screen_left, 0..=4095)); ui.end_row();
        ui.label("Topo:");     ui.add(egui::Slider::new(&mut m.screen_top, 0..=4095)); ui.end_row();
        ui.label("Direita:");  ui.add(egui::Slider::new(&mut m.screen_right, 0..=4095)); ui.end_row();
        ui.label("Base:");     ui.add(egui::Slider::new(&mut m.screen_bottom, 0..=4095)); ui.end_row();
    });

    ui.add_space(12.0);
    ui.checkbox(&mut m.force_proportions, "Forçar proporções (evitar distorção)");

    ui.add_space(8.0);
    ui.heading("Rotação");
    ui.horizontal(|ui| {
        ui.radio_value(&mut m.rotation, 0, "0°");
        ui.radio_value(&mut m.rotation, 90, "90°");
        ui.radio_value(&mut m.rotation, 180, "180°");
        ui.radio_value(&mut m.rotation, 270, "270°");
    });

    // Preview
    ui.separator();
    ui.label("Preview:");
    let ps = Vec2::new(250.0, 250.0);
    let (resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
    let rect = resp.rect;
    painter.rect_filled(rect, 4.0, Color32::from_gray(25));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);

    let tl = Pos2::new(
        rect.left() + (m.tablet_left as f32 / 4095.0) * rect.width(),
        rect.top() + (m.tablet_top as f32 / 4095.0) * rect.height(),
    );
    let br = Pos2::new(
        rect.left() + (m.tablet_right as f32 / 4095.0) * rect.width(),
        rect.top() + (m.tablet_bottom as f32 / 4095.0) * rect.height(),
    );
    painter.rect_filled(egui::Rect::from_min_max(tl, br), 2.0, Color32::from_rgba_premultiplied(80, 120, 200, 40));
    painter.rect_stroke(egui::Rect::from_min_max(tl, br), 2.0, Stroke::new(2.0, Color32::from_rgb(80, 120, 200)), StrokeKind::Outside);
    painter.text(Pos2::new(tl.x + 4.0, tl.y + 4.0), egui::Align2::LEFT_TOP, "Área ativa", egui::FontId::proportional(11.0), Color32::from_rgb(150, 180, 255));
}

// ── Orientation ──────────────────────────────────────────────────────────

pub fn orientation_tab(ui: &mut egui::Ui, o: &mut OrientationConfig) {
    ui.heading("Orientação");
    ui.separator();

    ui.checkbox(&mut o.left_hand, "Modo canhoto (left-hand mode)");
    ui.label("Espelha o eixo X para uso com mão esquerda.");

    ui.add_space(12.0);
    ui.checkbox(&mut o.flip_x, "Espelhar eixo X (flip horizontal)");
    ui.checkbox(&mut o.flip_y, "Espelhar eixo Y (flip vertical)");

    ui.separator();
    ui.label("Preview da orientação:");

    let ps = Vec2::new(200.0, 200.0);
    let (resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
    let rect = resp.rect;
    painter.rect_filled(rect, 4.0, Color32::from_gray(25));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);

    let flip_x = o.flip_x ^ o.left_hand;

    let arrow_start = Pos2::new(
        if flip_x { rect.right() - 20.0 } else { rect.left() + 20.0 },
        if o.flip_y { rect.bottom() - 20.0 } else { rect.top() + 20.0 },
    );
    let arrow_end_x = Pos2::new(
        if flip_x { rect.left() + 40.0 } else { rect.right() - 40.0 },
        arrow_start.y,
    );
    let arrow_end_y = Pos2::new(
        arrow_start.x,
        if o.flip_y { rect.top() + 40.0 } else { rect.bottom() - 40.0 },
    );

    painter.arrow(arrow_start, arrow_end_x - arrow_start, Stroke::new(2.0, Color32::from_rgb(255, 100, 100)));
    painter.arrow(arrow_start, arrow_end_y - arrow_start, Stroke::new(2.0, Color32::from_rgb(100, 255, 100)));

    painter.text(arrow_end_x, egui::Align2::CENTER_BOTTOM, "X", egui::FontId::proportional(14.0), Color32::from_rgb(255, 100, 100));
    painter.text(arrow_end_y, egui::Align2::LEFT_CENTER, "Y", egui::FontId::proportional(14.0), Color32::from_rgb(100, 255, 100));
}

// ── Pressure ─────────────────────────────────────────────────────────────

pub fn pressure_tab(ui: &mut egui::Ui, p: &mut PressureConfig) {
    ui.heading("Configuração de Pressão");
    ui.separator();

    egui::Grid::new("pressure_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Limiar de toque:");
        ui.add(egui::Slider::new(&mut p.touch_threshold, 500..=3000).text("raw"));
        ui.end_row();

        ui.label("Faixa de pressão:");
        ui.add(egui::Slider::new(&mut p.pressure_range, 50..=2000).text("raw"));
        ui.end_row();

        ui.label("Pressão máx reportada:");
        ui.add(egui::Slider::new(&mut p.max_pressure, 1024..=65535));
        ui.end_row();

        ui.label("Dead zone mínima:");
        ui.add(egui::Slider::new(&mut p.min_pressure_threshold, 0..=5000).text("evdev"));
        ui.end_row();
    });

    ui.separator();
    ui.heading("Curva de Pressão");
    ui.horizontal(|ui| {
        ui.radio_value(&mut p.curve, "linear".into(), "Linear");
        ui.radio_value(&mut p.curve, "soft".into(), "Soft");
        ui.radio_value(&mut p.curve, "firm".into(), "Firm");
        ui.radio_value(&mut p.curve, "custom".into(), "Custom");
    });

    if p.curve == "custom" {
        ui.add(egui::Slider::new(&mut p.gamma, 0.1..=4.0).text("Gamma").logarithmic(true));
    }

    ui.separator();
    ui.label("Preview da curva:");
    let cs = Vec2::new(300.0, 180.0);
    let (resp, painter) = ui.allocate_painter(cs, egui::Sense::hover());
    let rect = resp.rect;
    painter.rect_filled(rect, 4.0, Color32::from_gray(25));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);

    let gamma = p.gamma_value();
    let steps = 100;
    let points: Vec<Pos2> = (0..=steps).map(|i| {
        let t = i as f32 / steps as f32;
        let out = t.powf(gamma);
        Pos2::new(rect.left() + t * rect.width(), rect.bottom() - out * rect.height())
    }).collect();

    for w in points.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(2.0, Color32::from_rgb(255, 180, 60)));
    }

    // Dead zone line
    if p.min_pressure_threshold > 0 {
        let dz = p.min_pressure_threshold as f32 / p.max_pressure as f32;
        let y = rect.bottom() - dz * rect.height();
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0, Color32::from_rgb(255, 80, 80)),
        );
        painter.text(Pos2::new(rect.right() - 4.0, y - 2.0), egui::Align2::RIGHT_BOTTOM, "dead zone", egui::FontId::proportional(9.0), Color32::from_rgb(255, 80, 80));
    }

    painter.text(rect.left_bottom() + Vec2::new(4.0, -4.0), egui::Align2::LEFT_BOTTOM, "Força física →", egui::FontId::proportional(10.0), Color32::from_gray(120));
    painter.text(rect.left_top() + Vec2::new(4.0, 4.0), egui::Align2::LEFT_TOP, "↑ Pressão digital", egui::FontId::proportional(10.0), Color32::from_gray(120));
}

// ── Smoothing ────────────────────────────────────────────────────────────

pub fn smoothing_tab(ui: &mut egui::Ui, s: &mut SmoothingConfig) {
    ui.heading("Suavização & Anti-Jitter");
    ui.separator();

    ui.checkbox(&mut s.enabled, "Habilitar suavização de traço");
    if s.enabled {
        ui.add(egui::Slider::new(&mut s.level, 1..=10).text("Nível"));
        ui.label("1 = mínimo (quase raw), 10 = máximo (muito suave)");
    }

    ui.add_space(12.0);
    ui.checkbox(&mut s.anti_chatter, "Anti-chatter (previne micro-toques ao levantar a caneta)");
    if s.anti_chatter {
        ui.add(egui::Slider::new(&mut s.anti_chatter_threshold, 1..=10).text("Frames de confirmação"));
        ui.label("Quantos frames consecutivos precisa pra confirmar mudança de toque.");
    }

    ui.separator();
    ui.label("Preview (o nível de suavização afeta o quão 'arrastado' o traço fica):");

    let ps = Vec2::new(300.0, 100.0);
    let (_resp, painter) = ui.allocate_painter(ps, egui::Sense::hover());
    let rect = _resp.rect;
    painter.rect_filled(rect, 4.0, Color32::from_gray(25));

    // Draw a simulated jagged line vs smoothed
    let alpha = if s.enabled { 1.0 / s.level.max(1) as f32 } else { 1.0 };
    let mut raw_points = Vec::new();
    let mut smooth_points = Vec::new();
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;

    for i in 0..100 {
        let t = i as f32 / 99.0;
        let raw_y = (t * 12.0).sin() * 20.0 + ((t * 37.0).sin() * 8.0);
        let px = rect.left() + t * rect.width();
        let py = rect.center().y + raw_y;
        raw_points.push(Pos2::new(px, py));

        if i == 0 { sx = px; sy = py; }
        sx += (px - sx) * alpha;
        sy += (py - sy) * alpha;
        smooth_points.push(Pos2::new(sx, sy));
    }

    for w in raw_points.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 100, 100, 80)));
    }
    for w in smooth_points.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(2.0, Color32::from_rgb(100, 200, 255)));
    }
}

// ── Buttons ──────────────────────────────────────────────────────────────

pub fn buttons_tab(ui: &mut egui::Ui, buttons: &mut ButtonConfig, pen: &mut PenButtonConfig) {
    ui.heading("Mapeamento de Botões");
    ui.separator();

    ui.heading("Botões da caneta");
    egui::Grid::new("pen_buttons").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Inferior (stylus):");
        ui.text_edit_singleline(&mut pen.stylus);
        ui.end_row();
        ui.label("Superior (eraser):");
        ui.text_edit_singleline(&mut pen.eraser);
        ui.end_row();
    });

    ui.add_space(12.0);
    ui.heading("Botões express da mesa");
    ui.label("Teclas separadas por vírgula. Ex: E / LEFTCTRL, Z / LEFTCTRL, LEFTSHIFT, S");

    let labels = ["B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12"];
    let defaults = ["E", "B", "Ctrl+-", "Ctrl++", "[", "]", "ScrollUp", "Tab", "ScrollDown", "Space", "Ctrl", "Alt"];

    egui::Grid::new("tablet_buttons").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
        for (i, (label, default)) in labels.iter().zip(defaults.iter()).enumerate() {
            ui.label(format!("{}:", label));

            let keys = buttons.keys_for_mut(i);
            let mut text = keys.join(", ");
            if ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut text)).changed() {
                *keys = text.split(',').map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty()).collect();
            }

            ui.label(egui::RichText::new(format!("({})", default)).small().color(Color32::from_gray(100)));
            ui.end_row();
        }
    });
}
