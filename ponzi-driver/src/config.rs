use evdev::KeyCode;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub device: DeviceConfig,
    pub tablet: TabletConfig,
    pub mapping: MappingConfig,
    pub orientation: OrientationConfig,
    pub pressure: PressureConfig,
    pub smoothing: SmoothingConfig,
    pub buttons: ButtonConfig,
    pub pen_buttons: PenButtonConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DeviceConfig {
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TabletConfig {
    pub resolution_x: i32,
    pub resolution_y: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum InputMode {
    Absolute,
    Relative,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct MappingConfig {
    pub mode: InputMode,
    pub tablet_left: i32,
    pub tablet_top: i32,
    pub tablet_right: i32,
    pub tablet_bottom: i32,
    pub screen_left: i32,
    pub screen_top: i32,
    pub screen_right: i32,
    pub screen_bottom: i32,
    pub force_proportions: bool,
    pub rotation: i32,
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct OrientationConfig {
    pub flip_x: bool,
    pub flip_y: bool,
    pub left_hand: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PressureConfig {
    pub touch_threshold: i32,
    pub pressure_range: i32,
    pub max_pressure: i32,
    pub min_pressure_threshold: i32,
    pub curve: String,
    pub gamma: f32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SmoothingConfig {
    pub enabled: bool,
    pub level: i32,
    pub anti_chatter: bool,
    pub anti_chatter_threshold: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ButtonConfig {
    pub b1: Vec<String>,
    pub b2: Vec<String>,
    pub b3: Vec<String>,
    pub b4: Vec<String>,
    pub b5: Vec<String>,
    pub b6: Vec<String>,
    pub b7: Vec<String>,
    pub b8: Vec<String>,
    pub b9: Vec<String>,
    pub b10: Vec<String>,
    pub b11: Vec<String>,
    pub b12: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PenButtonConfig {
    pub stylus: Vec<String>,
    pub eraser: Vec<String>,
    pub tip: Vec<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    #[must_use] 
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: DeviceConfig {
                vendor_id: 0x08F2,
                product_id: 0x6811,
            },
            tablet: TabletConfig {
                resolution_x: 4096,
                resolution_y: 4096,
            },
            mapping: MappingConfig {
                mode: InputMode::Absolute,
                tablet_left: 0,
                tablet_top: 0,
                tablet_right: 4095,
                tablet_bottom: 4095,
                screen_left: 0,
                screen_top: 0,
                screen_right: 4095,
                screen_bottom: 4095,
                force_proportions: false,
                rotation: 0,
                sensitivity_x: 1.0,
                sensitivity_y: 1.0,
            },
            orientation: OrientationConfig {
                flip_x: false,
                flip_y: false,
                left_hand: false,
            },
            pressure: PressureConfig {
                touch_threshold: 1510,
                pressure_range: 530,
                max_pressure: 65535,
                min_pressure_threshold: 0,
                curve: "linear".into(),
                gamma: 1.0,
            },
            smoothing: SmoothingConfig {
                enabled: false,
                level: 3,
                anti_chatter: false,
                anti_chatter_threshold: 2,
            },
            buttons: ButtonConfig {
                b1: vec!["LEFTCTRL".into(), "Z".into()],           // Undo
                b2: vec!["LEFTCTRL".into(), "LEFTSHIFT".into(), "Z".into()], // Redo
                b3: vec!["P".into()],                               // Pen/Draw
                b4: vec!["E".into()],                               // Eraser
                b5: vec!["V".into()],                               // Select
                b6: vec!["H".into()],                               // Hand/Pan
                b7: vec!["DELETE".into()],                          // Delete
                b8: vec!["LEFTCTRL".into(), "D".into()],           // Duplicate
                b9: vec!["LEFTCTRL".into(), "A".into()],           // Select all
                b10: vec!["LEFTSHIFT".into(), "1".into()],         // Zoom to fit
                b11: vec!["S".into()],                              // Stroke color
                b12: vec!["R".into()],                              // Rectangle
            },
            pen_buttons: PenButtonConfig {
                stylus: vec!["LEFTCTRL".into(), "LEFTSHIFT".into(), "EQUAL".into()],
                eraser: vec!["LEFTCTRL".into(), "MINUS".into()],
                tip: vec!["BTN_LEFT".into()],
            },
        }
    }
}

impl MappingConfig {
    #[must_use] 
    pub fn tablet_width(&self) -> i32 {
        self.tablet_right - self.tablet_left
    }

    #[must_use] 
    pub fn tablet_height(&self) -> i32 {
        self.tablet_bottom - self.tablet_top
    }

    #[must_use] 
    pub fn screen_width(&self) -> i32 {
        self.screen_right - self.screen_left
    }

    #[must_use] 
    pub fn screen_height(&self) -> i32 {
        self.screen_bottom - self.screen_top
    }
}

impl PressureConfig {
    #[must_use] 
    pub fn gamma_value(&self) -> f32 {
        match self.curve.as_str() {
            "soft" => 0.5,
            "firm" => 2.0,
            "custom" => self.gamma,
            _ => 1.0,
        }
    }
}

impl ButtonConfig {
    #[must_use] 
    pub fn keys_for(&self, index: usize) -> &[String] {
        match index {
            0 => &self.b1,
            1 => &self.b2,
            2 => &self.b3,
            3 => &self.b4,
            4 => &self.b5,
            5 => &self.b6,
            6 => &self.b7,
            7 => &self.b8,
            8 => &self.b9,
            9 => &self.b10,
            10 => &self.b11,
            11 => &self.b12,
            _ => &[],
        }
    }

    pub fn keys_for_mut(&mut self, index: usize) -> &mut Vec<String> {
        match index {
            0 => &mut self.b1,
            1 => &mut self.b2,
            2 => &mut self.b3,
            3 => &mut self.b4,
            4 => &mut self.b5,
            5 => &mut self.b6,
            6 => &mut self.b7,
            7 => &mut self.b8,
            8 => &mut self.b9,
            9 => &mut self.b10,
            10 => &mut self.b11,
            11 => &mut self.b12,
            _ => unreachable!(),
        }
    }
}

/// Parse a key name like "LEFTCTRL" or "`BTN_STYLUS`" into an evdev `KeyCode`.
#[must_use] 
pub fn parse_key(name: &str) -> Option<KeyCode> {
    let candidates = [
        format!("KEY_{name}"),
        name.to_string(),
    ];
    for candidate in &candidates {
        if let Some(kc) = keycode_from_name(candidate) {
            return Some(kc);
        }
    }
    None
}

fn keycode_from_name(name: &str) -> Option<KeyCode> {
    match name {
        "KEY_E" => Some(KeyCode::KEY_E),
        "KEY_B" => Some(KeyCode::KEY_B),
        "KEY_TAB" => Some(KeyCode::KEY_TAB),
        "KEY_SPACE" => Some(KeyCode::KEY_SPACE),
        "KEY_LEFTCTRL" => Some(KeyCode::KEY_LEFTCTRL),
        "KEY_RIGHTCTRL" => Some(KeyCode::KEY_RIGHTCTRL),
        "KEY_LEFTALT" => Some(KeyCode::KEY_LEFTALT),
        "KEY_RIGHTALT" => Some(KeyCode::KEY_RIGHTALT),
        "KEY_LEFTSHIFT" => Some(KeyCode::KEY_LEFTSHIFT),
        "KEY_RIGHTSHIFT" => Some(KeyCode::KEY_RIGHTSHIFT),
        "KEY_LEFTMETA" => Some(KeyCode::KEY_LEFTMETA),
        "KEY_RIGHTMETA" => Some(KeyCode::KEY_RIGHTMETA),
        "KEY_LEFTBRACE" => Some(KeyCode::KEY_LEFTBRACE),
        "KEY_RIGHTBRACE" => Some(KeyCode::KEY_RIGHTBRACE),
        "KEY_KPMINUS" => Some(KeyCode::KEY_KPMINUS),
        "KEY_KPPLUS" => Some(KeyCode::KEY_KPPLUS),
        "KEY_SCROLLUP" => Some(KeyCode::KEY_SCROLLUP),
        "KEY_SCROLLDOWN" => Some(KeyCode::KEY_SCROLLDOWN),
        "KEY_A" => Some(KeyCode::KEY_A),
        "KEY_C" => Some(KeyCode::KEY_C),
        "KEY_D" => Some(KeyCode::KEY_D),
        "KEY_F" => Some(KeyCode::KEY_F),
        "KEY_G" => Some(KeyCode::KEY_G),
        "KEY_H" => Some(KeyCode::KEY_H),
        "KEY_I" => Some(KeyCode::KEY_I),
        "KEY_J" => Some(KeyCode::KEY_J),
        "KEY_K" => Some(KeyCode::KEY_K),
        "KEY_L" => Some(KeyCode::KEY_L),
        "KEY_M" => Some(KeyCode::KEY_M),
        "KEY_N" => Some(KeyCode::KEY_N),
        "KEY_O" => Some(KeyCode::KEY_O),
        "KEY_P" => Some(KeyCode::KEY_P),
        "KEY_Q" => Some(KeyCode::KEY_Q),
        "KEY_R" => Some(KeyCode::KEY_R),
        "KEY_S" => Some(KeyCode::KEY_S),
        "KEY_T" => Some(KeyCode::KEY_T),
        "KEY_U" => Some(KeyCode::KEY_U),
        "KEY_V" => Some(KeyCode::KEY_V),
        "KEY_W" => Some(KeyCode::KEY_W),
        "KEY_X" => Some(KeyCode::KEY_X),
        "KEY_Y" => Some(KeyCode::KEY_Y),
        "KEY_Z" => Some(KeyCode::KEY_Z),
        "KEY_0" => Some(KeyCode::KEY_0),
        "KEY_1" => Some(KeyCode::KEY_1),
        "KEY_2" => Some(KeyCode::KEY_2),
        "KEY_3" => Some(KeyCode::KEY_3),
        "KEY_4" => Some(KeyCode::KEY_4),
        "KEY_5" => Some(KeyCode::KEY_5),
        "KEY_6" => Some(KeyCode::KEY_6),
        "KEY_7" => Some(KeyCode::KEY_7),
        "KEY_8" => Some(KeyCode::KEY_8),
        "KEY_9" => Some(KeyCode::KEY_9),
        "KEY_F1" => Some(KeyCode::KEY_F1),
        "KEY_F2" => Some(KeyCode::KEY_F2),
        "KEY_F3" => Some(KeyCode::KEY_F3),
        "KEY_F4" => Some(KeyCode::KEY_F4),
        "KEY_F5" => Some(KeyCode::KEY_F5),
        "KEY_F6" => Some(KeyCode::KEY_F6),
        "KEY_F7" => Some(KeyCode::KEY_F7),
        "KEY_F8" => Some(KeyCode::KEY_F8),
        "KEY_F9" => Some(KeyCode::KEY_F9),
        "KEY_F10" => Some(KeyCode::KEY_F10),
        "KEY_F11" => Some(KeyCode::KEY_F11),
        "KEY_F12" => Some(KeyCode::KEY_F12),
        "KEY_ESC" => Some(KeyCode::KEY_ESC),
        "KEY_ENTER" => Some(KeyCode::KEY_ENTER),
        "KEY_BACKSPACE" => Some(KeyCode::KEY_BACKSPACE),
        "KEY_DELETE" => Some(KeyCode::KEY_DELETE),
        "KEY_HOME" => Some(KeyCode::KEY_HOME),
        "KEY_END" => Some(KeyCode::KEY_END),
        "KEY_PAGEUP" => Some(KeyCode::KEY_PAGEUP),
        "KEY_PAGEDOWN" => Some(KeyCode::KEY_PAGEDOWN),
        "KEY_UP" => Some(KeyCode::KEY_UP),
        "KEY_DOWN" => Some(KeyCode::KEY_DOWN),
        "KEY_LEFT" => Some(KeyCode::KEY_LEFT),
        "KEY_RIGHT" => Some(KeyCode::KEY_RIGHT),
        "KEY_MINUS" => Some(KeyCode::KEY_MINUS),
        "KEY_EQUAL" => Some(KeyCode::KEY_EQUAL),
        "KEY_COMMA" => Some(KeyCode::KEY_COMMA),
        "KEY_DOT" => Some(KeyCode::KEY_DOT),
        "KEY_SLASH" => Some(KeyCode::KEY_SLASH),
        "KEY_BACKSLASH" => Some(KeyCode::KEY_BACKSLASH),
        "KEY_SEMICOLON" => Some(KeyCode::KEY_SEMICOLON),
        "KEY_APOSTROPHE" => Some(KeyCode::KEY_APOSTROPHE),
        "KEY_GRAVE" => Some(KeyCode::KEY_GRAVE),
        "BTN_STYLUS" => Some(KeyCode::BTN_STYLUS),
        "BTN_STYLUS2" => Some(KeyCode::BTN_STYLUS2),
        "BTN_TOUCH" => Some(KeyCode::BTN_TOUCH),
        "BTN_LEFT" => Some(KeyCode::BTN_LEFT),
        "BTN_RIGHT" => Some(KeyCode::BTN_RIGHT),
        "BTN_MIDDLE" => Some(KeyCode::BTN_MIDDLE),
        "BTN_TOOL_PEN" => Some(KeyCode::BTN_TOOL_PEN),
        "BTN_TOOL_RUBBER" => Some(KeyCode::BTN_TOOL_RUBBER),
        _ => None,
    }
}
