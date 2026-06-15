use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, EventType, InputEvent, InputId,
    KeyCode, KeyEvent, PropType, UinputAbsSetup,
    uinput::VirtualDevice,
};
use log::{error, info, warn};
use std::io;

use crate::config::{ButtonConfig, Config, MappingConfig, OrientationConfig, PressureConfig, SmoothingConfig, parse_key};
use crate::protocol::{PenButton, PenData, TabletKey};

pub struct VirtualPen {
    device: VirtualDevice,
    pen_in_range: bool,
}

pub struct VirtualKeys {
    device: VirtualDevice,
}

impl VirtualPen {
    pub fn new(pressure: &PressureConfig, _mapping: &MappingConfig) -> io::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_TOOL_PEN);
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_STYLUS);
        keys.insert(KeyCode::BTN_STYLUS2);

        let x = UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, AbsInfo::new(0, 0, 4096, 0, 0, 1));
        let y = UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, AbsInfo::new(0, 0, 4096, 0, 0, 1));
        let press = UinputAbsSetup::new(AbsoluteAxisCode::ABS_PRESSURE, AbsInfo::new(0, 0, pressure.max_pressure, 0, 0, 1));

        let device = VirtualDevice::builder()?
            .name("Ponzi Tablet Pen")
            .with_absolute_axis(&x)?
            .with_absolute_axis(&y)?
            .with_absolute_axis(&press)?
            .with_keys(&keys)?
            .build()?;

        info!("Virtual pen device created (range 0-4096)");

        Ok(Self { device, pen_in_range: false })
    }

    pub fn emit(&mut self, events: &[InputEvent]) {
        if events.is_empty() {
            return;
        }
        if let Err(e) = self.device.emit(events) {
            error!("Failed to emit pen events: {}", e);
        }
    }

    pub fn set_in_range(&mut self, in_range: bool) {
        if in_range != self.pen_in_range {
            self.pen_in_range = in_range;
            let val = if in_range { 1 } else { 0 };
            let event = KeyEvent::new(KeyCode::BTN_TOOL_PEN, val);
            self.emit(&[event.into()]);
        }
    }
}

impl VirtualKeys {
    pub fn new(button_cfg: &ButtonConfig) -> io::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for i in 0..12 {
            for name in button_cfg.keys_for(i) {
                if let Some(kc) = parse_key(name) {
                    keys.insert(kc);
                }
            }
        }

        let device = VirtualDevice::builder()?
            .name("Ponzi Tablet Buttons")
            .with_keys(&keys)?
            .build()?;

        info!("Virtual keyboard device created");

        Ok(Self { device })
    }

    pub fn emit(&mut self, events: &[InputEvent]) {
        if events.is_empty() {
            return;
        }
        if let Err(e) = self.device.emit(events) {
            error!("Failed to emit key events: {}", e);
        }
    }
}

fn map_coordinate(raw: i32, tablet_min: i32, tablet_max: i32, screen_min: i32, screen_max: i32) -> i32 {
    let clamped = raw.clamp(tablet_min, tablet_max);
    let tablet_range = tablet_max - tablet_min;
    let screen_range = screen_max - screen_min;
    if tablet_range == 0 {
        return screen_min;
    }
    screen_min + ((clamped - tablet_min) as i64 * screen_range as i64 / tablet_range as i64) as i32
}

fn normalize_pressure(raw: i32, cfg: &PressureConfig) -> i32 {
    if raw >= cfg.touch_threshold {
        return 0;
    }
    let linear = ((cfg.touch_threshold - raw) as f32 / cfg.pressure_range as f32).clamp(0.0, 1.0);
    let gamma = cfg.gamma_value();
    let curved = linear.powf(gamma);
    let result = (curved * cfg.max_pressure as f32).clamp(0.0, cfg.max_pressure as f32) as i32;
    if result < cfg.min_pressure_threshold {
        return 0;
    }
    result
}

fn is_touching(raw_pressure: i32, threshold: i32) -> bool {
    raw_pressure < threshold
}

fn apply_orientation(x: i32, y: i32, res: i32, orient: &OrientationConfig, rotation: i32) -> (i32, i32) {
    let (mut ox, mut oy) = match rotation {
        90 => (res - y, x),
        180 => (res - x, res - y),
        270 => (y, res - x),
        _ => (x, y),
    };

    let left_hand = orient.left_hand;
    let flip_x = orient.flip_x ^ left_hand;
    let flip_y = orient.flip_y;

    if flip_x { ox = res - ox; }
    if flip_y { oy = res - oy; }

    (ox, oy)
}

pub struct InputProcessor {
    prev: PenData,
    prev_touching: bool,
    prev_tablet_buttons: Vec<TabletKey>,
    prev_pen_button: Option<PenButton>,
    pub mapping: MappingConfig,
    pub orientation: OrientationConfig,
    pressure: PressureConfig,
    smoothing: SmoothingConfig,
    resolution: i32,
    button_keys: [Vec<KeyCode>; 12],
    stylus_key: KeyCode,
    eraser_key: KeyCode,
    // Smoothing state
    smooth_x: f32,
    smooth_y: f32,
    smooth_initialized: bool,
    // Anti-chatter
    chatter_count: i32,
}

impl InputProcessor {
    pub fn new(cfg: &Config) -> Self {
        let mut button_keys: [Vec<KeyCode>; 12] = Default::default();
        for i in 0..12 {
            for name in cfg.buttons.keys_for(i) {
                match parse_key(name) {
                    Some(kc) => button_keys[i].push(kc),
                    None => warn!("Unknown key '{}' in b{}", name, i + 1),
                }
            }
        }

        let stylus_key = parse_key(&cfg.pen_buttons.stylus).unwrap_or(KeyCode::BTN_STYLUS);
        let eraser_key = parse_key(&cfg.pen_buttons.eraser).unwrap_or(KeyCode::BTN_STYLUS2);

        Self {
            prev: PenData::default(),
            prev_touching: false,
            prev_tablet_buttons: Vec::new(),
            prev_pen_button: None,
            mapping: cfg.mapping.clone(),
            orientation: cfg.orientation.clone(),
            pressure: cfg.pressure.clone(),
            smoothing: cfg.smoothing.clone(),
            resolution: cfg.tablet.resolution_x,
            button_keys,
            stylus_key,
            eraser_key,
            smooth_x: 0.0,
            smooth_y: 0.0,
            smooth_initialized: false,
            chatter_count: 0,
        }
    }

    pub fn process(&mut self, data: &PenData, pen: &mut VirtualPen, keys: &mut VirtualKeys) {
        let mut pen_events: Vec<InputEvent> = Vec::new();
        let mut key_events: Vec<InputEvent> = Vec::new();

        // Signal pen in range (any valid data = pen is near the surface)
        let has_position = data.x > 0 || data.y > 0 || data.pressure_raw > 0;
        pen.set_in_range(has_position);

        // Apply orientation (rotation + flip)
        let (ox, oy) = apply_orientation(
            data.x, data.y, self.resolution,
            &self.orientation, self.mapping.rotation,
        );

        // Smoothing
        let (sx, sy) = if self.smoothing.enabled {
            let alpha = 1.0 / (self.smoothing.level.max(1) as f32);
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

        // Check if pen is inside the mapped tablet area
        let in_bounds = sx >= self.mapping.tablet_left
            && sx <= self.mapping.tablet_right
            && sy >= self.mapping.tablet_top
            && sy <= self.mapping.tablet_bottom;

        if in_bounds {
            let mapped_x = map_coordinate(sx, self.mapping.tablet_left, self.mapping.tablet_right, self.mapping.screen_left, self.mapping.screen_right);
            let mapped_y = map_coordinate(sy, self.mapping.tablet_top, self.mapping.tablet_bottom, self.mapping.screen_top, self.mapping.screen_bottom);

            let (prev_ox, prev_oy) = apply_orientation(
                self.prev.x, self.prev.y, self.resolution,
                &self.orientation, self.mapping.rotation,
            );
            let prev_mx = map_coordinate(prev_ox, self.mapping.tablet_left, self.mapping.tablet_right, self.mapping.screen_left, self.mapping.screen_right);
            let prev_my = map_coordinate(prev_oy, self.mapping.tablet_top, self.mapping.tablet_bottom, self.mapping.screen_top, self.mapping.screen_bottom);

            if prev_mx != mapped_x || prev_my != mapped_y {
                pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, mapped_x));
                pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, mapped_y));
            }
        }

        // Pressure (only emit if in bounds)
        if self.prev.pressure_raw != data.pressure_raw {
            let p = if in_bounds { normalize_pressure(data.pressure_raw, &self.pressure) } else { 0 };
            pen_events.push(InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_PRESSURE.0, p));
        }

        // Touch state — only register touch if in bounds
        let raw_touching = is_touching(data.pressure_raw, self.pressure.touch_threshold);
        let touching = raw_touching && in_bounds;
        let emit_touch_change = if self.smoothing.anti_chatter {
            if touching != self.prev_touching {
                self.chatter_count += 1;
                self.chatter_count >= self.smoothing.anti_chatter_threshold
            } else {
                self.chatter_count = 0;
                false
            }
        } else {
            touching != self.prev_touching
        };

        if emit_touch_change {
            if touching {
                pen_events.push(KeyEvent::new(KeyCode::BTN_TOUCH, 1).into());
                pen_events.push(KeyEvent::new(KeyCode::BTN_LEFT, 1).into());
            } else {
                pen_events.push(KeyEvent::new(KeyCode::BTN_TOUCH, 0).into());
                pen_events.push(KeyEvent::new(KeyCode::BTN_LEFT, 0).into());
                self.smooth_initialized = false;
            }
            self.chatter_count = 0;
        }

        // Pen buttons
        let cur_pen = PenButton::from_raw(data.pen_button);
        if cur_pen != self.prev_pen_button {
            if let Some(prev) = self.prev_pen_button {
                pen_events.push(KeyEvent::new(self.pen_kc(prev), 0).into());
            }
            if let Some(curr) = cur_pen {
                pen_events.push(KeyEvent::new(self.pen_kc(curr), 1).into());
            }
        }

        // Tablet express keys
        let mut cur_buttons = Vec::new();
        for &btn in TabletKey::ALL {
            if btn.is_pressed(data.tablet_buttons) {
                cur_buttons.push(btn);
            }
        }
        for &btn in &cur_buttons {
            if !self.prev_tablet_buttons.contains(&btn) {
                for &kc in &self.button_keys[tablet_key_index(btn)] {
                    key_events.push(KeyEvent::new(kc, 1).into());
                }
            }
        }
        for &btn in &self.prev_tablet_buttons {
            if !cur_buttons.contains(&btn) {
                for &kc in &self.button_keys[tablet_key_index(btn)] {
                    key_events.push(KeyEvent::new(kc, 0).into());
                }
            }
        }

        self.prev = *data;
        if emit_touch_change { self.prev_touching = touching; }
        self.prev_pen_button = cur_pen;
        self.prev_tablet_buttons = cur_buttons;

        pen.emit(&pen_events);
        keys.emit(&key_events);
    }

    fn pen_kc(&self, btn: PenButton) -> KeyCode {
        match btn {
            PenButton::Stylus => self.stylus_key,
            PenButton::Eraser => self.eraser_key,
        }
    }
}

fn tablet_key_index(key: TabletKey) -> usize {
    match key {
        TabletKey::B1 => 0,  TabletKey::B2 => 1,  TabletKey::B3 => 2,
        TabletKey::B4 => 3,  TabletKey::B5 => 4,  TabletKey::B6 => 5,
        TabletKey::B7 => 6,  TabletKey::B8 => 7,  TabletKey::B9 => 8,
        TabletKey::B10 => 9, TabletKey::B11 => 10, TabletKey::B12 => 11,
    }
}
