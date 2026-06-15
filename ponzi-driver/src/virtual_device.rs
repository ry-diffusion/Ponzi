use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, EventType, InputEvent, InputId,
    KeyCode, KeyEvent, PropType, RelativeAxisCode, UinputAbsSetup,
    uinput::VirtualDevice,
};
use log::{error, info, warn};
use std::io;
use std::sync::{Arc, Mutex};

use crate::config::{ButtonConfig, Config, InputMode, parse_key};
use crate::protocol::{PenButton, PenData, TabletKey};

pub struct VirtualPen {
    device: VirtualDevice,
    pen_in_range: bool,
    pub mode: InputMode,
}

pub struct VirtualKeys {
    device: VirtualDevice,
}

impl VirtualPen {
    pub fn new(cfg: &Config) -> io::Result<Self> {
        let mode = cfg.mapping.mode.clone();

        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_TOOL_PEN);
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_STYLUS);
        keys.insert(KeyCode::BTN_STYLUS2);
        // Register all keys used by pen buttons
        for name in cfg.pen_buttons.tip.iter()
            .chain(cfg.pen_buttons.stylus.iter())
            .chain(cfg.pen_buttons.eraser.iter()) {
            if let Some(kc) = parse_key(name) { keys.insert(kc); }
        }

        let press = UinputAbsSetup::new(AbsoluteAxisCode::ABS_PRESSURE, AbsInfo::new(0, 0, cfg.pressure.max_pressure, 0, 0, 1));

        let device = match mode {
            InputMode::Absolute => {
                let x = UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, AbsInfo::new(0, 0, 4096, 0, 0, 1));
                let y = UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, AbsInfo::new(0, 0, 4096, 0, 0, 1));
                VirtualDevice::builder()?
                    .name("Ponzi Tablet Pen")
                    .with_absolute_axis(&x)?
                    .with_absolute_axis(&y)?
                    .with_absolute_axis(&press)?
                    .with_keys(&keys)?
                    .build()?
            }
            InputMode::Relative => {
                let mut rel = AttributeSet::<RelativeAxisCode>::new();
                rel.insert(RelativeAxisCode::REL_X);
                rel.insert(RelativeAxisCode::REL_Y);
                VirtualDevice::builder()?
                    .name("Ponzi Tablet Pen (Relative)")
                    .with_relative_axes(&rel)?
                    .with_absolute_axis(&press)?
                    .with_keys(&keys)?
                    .build()?
            }
        };

        info!("Virtual pen device created (mode: {:?})", mode);
        Ok(Self { device, pen_in_range: false, mode })
    }

    pub fn emit(&mut self, events: &[InputEvent]) {
        if events.is_empty() { return; }
        if let Err(e) = self.device.emit(events) {
            error!("Failed to emit pen events: {}", e);
        }
    }

    pub fn set_in_range(&mut self, in_range: bool) {
        if in_range != self.pen_in_range {
            self.pen_in_range = in_range;
            self.emit(&[KeyEvent::new(KeyCode::BTN_TOOL_PEN, if in_range { 1 } else { 0 }).into()]);
        }
    }
}

impl VirtualKeys {
    pub fn new(cfg: &Config) -> io::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for i in 0..12 {
            for name in cfg.buttons.keys_for(i) {
                if let Some(kc) = parse_key(name) { keys.insert(kc); }
            }
        }
        // Register pen barrel button keys so they emit from keyboard device
        for name in cfg.pen_buttons.stylus.iter()
            .chain(cfg.pen_buttons.eraser.iter())
            .chain(cfg.pen_buttons.tip.iter()) {
            if let Some(kc) = parse_key(name) { keys.insert(kc); }
        }

        let device = VirtualDevice::builder()?
            .name("Ponzi Tablet Buttons")
            .with_keys(&keys)?
            .build()?;

        info!("Virtual keyboard device created");
        Ok(Self { device })
    }

    pub fn emit(&mut self, events: &[InputEvent]) {
        if events.is_empty() { return; }
        if let Err(e) = self.device.emit(events) {
            error!("Failed to emit key events: {}", e);
        }
    }
}

fn map_coordinate(raw: i32, tablet_min: i32, tablet_max: i32, screen_min: i32, screen_max: i32) -> i32 {
    let clamped = raw.clamp(tablet_min, tablet_max);
    let tablet_range = tablet_max - tablet_min;
    let screen_range = screen_max - screen_min;
    if tablet_range == 0 { return screen_min; }
    screen_min + ((clamped - tablet_min) as i64 * screen_range as i64 / tablet_range as i64) as i32
}

fn normalize_pressure(raw: i32, threshold: i32, range: i32, max: i32, gamma: f32, dead_zone: i32) -> i32 {
    if raw >= threshold { return 0; }
    let linear = ((threshold - raw) as f32 / range as f32).clamp(0.0, 1.0);
    let curved = linear.powf(gamma);
    let result = (curved * max as f32).clamp(0.0, max as f32) as i32;
    if result < dead_zone { return 0; }
    result
}

fn is_touching(raw_pressure: i32, threshold: i32) -> bool {
    raw_pressure < threshold
}

fn apply_orientation(x: i32, y: i32, res: i32, flip_x: bool, flip_y: bool, left_hand: bool, rotation: i32) -> (i32, i32) {
    let (mut ox, mut oy) = match rotation {
        90 => (res - y, x),
        180 => (res - x, res - y),
        270 => (y, res - x),
        _ => (x, y),
    };
    if flip_x ^ left_hand { ox = res - ox; }
    if flip_y { oy = res - oy; }
    (ox, oy)
}

/// Reads config from shared Arc<Mutex<Config>> each frame — no stale copies.
pub struct InputProcessor {
    cfg: Arc<Mutex<Config>>,
    prev: PenData,
    prev_touching: bool,
    prev_tablet_buttons: Vec<TabletKey>,
    prev_pen_button: Option<PenButton>,
    button_keys: [Vec<KeyCode>; 12],
    stylus_keys: Vec<KeyCode>,
    eraser_keys: Vec<KeyCode>,
    tip_keys: Vec<KeyCode>,
    smooth_x: f32,
    smooth_y: f32,
    smooth_initialized: bool,
    chatter_count: i32,
}

impl InputProcessor {
    pub fn new(cfg: Arc<Mutex<Config>>) -> Self {
        let c = cfg.lock().unwrap();
        let mut button_keys: [Vec<KeyCode>; 12] = Default::default();
        for i in 0..12 {
            for name in c.buttons.keys_for(i) {
                match parse_key(name) {
                    Some(kc) => button_keys[i].push(kc),
                    None => warn!("Unknown key '{}' in b{}", name, i + 1),
                }
            }
        }
        let resolve = |keys: &[String]| -> Vec<KeyCode> {
            keys.iter().filter_map(|k| parse_key(k)).collect()
        };
        let stylus_keys = resolve(&c.pen_buttons.stylus);
        let eraser_keys = resolve(&c.pen_buttons.eraser);
        let tip_keys = resolve(&c.pen_buttons.tip);
        drop(c);

        Self {
            cfg,
            prev: PenData::default(),
            prev_touching: false,
            prev_tablet_buttons: Vec::new(),
            prev_pen_button: None,
            button_keys,
            stylus_keys, eraser_keys, tip_keys,
            smooth_x: 0.0, smooth_y: 0.0,
            smooth_initialized: false,
            chatter_count: 0,
        }
    }

    pub fn process(&mut self, data: &PenData, pen: &mut VirtualPen, keys: &mut VirtualKeys) {
        let cfg = self.cfg.lock().unwrap();
        let mapping = &cfg.mapping;
        let orient = &cfg.orientation;
        let pressure = &cfg.pressure;
        let smoothing = &cfg.smoothing;
        let resolution = cfg.tablet.resolution_x;

        let mut pen_events: Vec<InputEvent> = Vec::new();
        let mut key_events: Vec<InputEvent> = Vec::new();

        let has_position = data.x > 0 || data.y > 0 || data.pressure_raw > 0;
        pen.set_in_range(has_position);

        let (ox, oy) = apply_orientation(
            data.x, data.y, resolution,
            orient.flip_x, orient.flip_y, orient.left_hand, mapping.rotation,
        );

        let (sx, sy) = if smoothing.enabled {
            let alpha = 1.0 / (smoothing.level.max(1) as f32);
            if !self.smooth_initialized {
                self.smooth_x = ox as f32;
                self.smooth_y = oy as f32;
                self.smooth_initialized = true;
            }
            self.smooth_x += (ox as f32 - self.smooth_x) * alpha;
            self.smooth_y += (oy as f32 - self.smooth_y) * alpha;
            (self.smooth_x as i32, self.smooth_y as i32)
        } else {
            (ox, oy)
        };

        let in_bounds = sx >= mapping.tablet_left && sx <= mapping.tablet_right
            && sy >= mapping.tablet_top && sy <= mapping.tablet_bottom;

        match mapping.mode {
            InputMode::Absolute => {
                if in_bounds {
                    let mx = map_coordinate(sx, mapping.tablet_left, mapping.tablet_right, mapping.screen_left, mapping.screen_right);
                    let my = map_coordinate(sy, mapping.tablet_top, mapping.tablet_bottom, mapping.screen_top, mapping.screen_bottom);

                    let (prev_ox, prev_oy) = apply_orientation(
                        self.prev.x, self.prev.y, resolution,
                        orient.flip_x, orient.flip_y, orient.left_hand, mapping.rotation,
                    );
                    let prev_mx = map_coordinate(prev_ox, mapping.tablet_left, mapping.tablet_right, mapping.screen_left, mapping.screen_right);
                    let prev_my = map_coordinate(prev_oy, mapping.tablet_top, mapping.tablet_bottom, mapping.screen_top, mapping.screen_bottom);

                    if prev_mx != mx || prev_my != my {
                        pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, mx));
                        pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, my));
                    }
                }
            }
            InputMode::Relative => {
                let dx = sx - self.prev.x;
                let dy = sy - self.prev.y;
                if (dx != 0 || dy != 0) && self.prev.x != 0 {
                    let rel_x = (dx as f32 * mapping.sensitivity_x) as i32;
                    let rel_y = (dy as f32 * mapping.sensitivity_y) as i32;
                    if rel_x != 0 {
                        pen_events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, rel_x));
                    }
                    if rel_y != 0 {
                        pen_events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, rel_y));
                    }
                }
            }
        }

        if self.prev.pressure_raw != data.pressure_raw {
            let p = if in_bounds {
                normalize_pressure(data.pressure_raw, pressure.touch_threshold, pressure.pressure_range,
                    pressure.max_pressure, pressure.gamma_value(), pressure.min_pressure_threshold)
            } else { 0 };
            pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_PRESSURE.0, p));
        }

        let raw_touching = is_touching(data.pressure_raw, pressure.touch_threshold);
        let touching = raw_touching && in_bounds;
        let emit_touch = if smoothing.anti_chatter {
            if touching != self.prev_touching {
                self.chatter_count += 1;
                self.chatter_count >= smoothing.anti_chatter_threshold
            } else { self.chatter_count = 0; false }
        } else {
            touching != self.prev_touching
        };

        if emit_touch {
            let v = if touching { 1 } else { 0 };
            pen_events.push(KeyEvent::new(KeyCode::BTN_TOUCH, v).into());
            for &kc in &self.tip_keys {
                pen_events.push(KeyEvent::new(kc, v).into());
            }
            if !touching { self.smooth_initialized = false; }
            self.chatter_count = 0;
        }

        let cur_pen = PenButton::from_raw(data.pen_button);
        if cur_pen != self.prev_pen_button {
            if let Some(prev) = self.prev_pen_button {
                let ks = match prev { PenButton::Stylus => &self.stylus_keys, PenButton::Eraser => &self.eraser_keys };
                for &kc in ks { key_events.push(KeyEvent::new(kc, 0).into()); }
            }
            if let Some(curr) = cur_pen {
                let ks = match curr { PenButton::Stylus => &self.stylus_keys, PenButton::Eraser => &self.eraser_keys };
                for &kc in ks { key_events.push(KeyEvent::new(kc, 1).into()); }
            }
        }

        let mut cur_buttons = Vec::new();
        for &btn in TabletKey::ALL {
            if btn.is_pressed(data.tablet_buttons) { cur_buttons.push(btn); }
        }
        for &btn in &cur_buttons {
            if !self.prev_tablet_buttons.contains(&btn) {
                for &kc in &self.button_keys[btn_idx(btn)] { key_events.push(KeyEvent::new(kc, 1).into()); }
            }
        }
        for &btn in &self.prev_tablet_buttons {
            if !cur_buttons.contains(&btn) {
                for &kc in &self.button_keys[btn_idx(btn)] { key_events.push(KeyEvent::new(kc, 0).into()); }
            }
        }

        // Drop the lock before emitting (emit may block briefly)
        drop(cfg);

        self.prev = *data;
        if emit_touch { self.prev_touching = touching; }
        self.prev_pen_button = cur_pen;
        self.prev_tablet_buttons = cur_buttons;

        pen.emit(&pen_events);
        keys.emit(&key_events);
    }
}

fn btn_idx(key: TabletKey) -> usize {
    match key {
        TabletKey::B1 => 0,  TabletKey::B2 => 1,  TabletKey::B3 => 2,
        TabletKey::B4 => 3,  TabletKey::B5 => 4,  TabletKey::B6 => 5,
        TabletKey::B7 => 6,  TabletKey::B8 => 7,  TabletKey::B9 => 8,
        TabletKey::B10 => 9, TabletKey::B11 => 10, TabletKey::B12 => 11,
    }
}
