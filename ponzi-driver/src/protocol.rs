/// Raw input data decoded from a 64-byte USB interrupt transfer on EP 0x83.
///
/// Packet layout (from reverse engineering the Windows TabletService.exe
/// and the reference t1161-driver):
///
/// Offset  Size  Description
/// ------  ----  -----------
///  0       1    Report ID / status byte
///  1-2     2    X coordinate (big-endian u16, 0..4095)
///  3-4     2    Y coordinate (big-endian u16, 0..4095)
///  5-6     2    Pressure (big-endian u16, inverted: lower = harder press)
///  7-8     2    (reserved / tilt on some models)
///  9       1    Pen barrel buttons (4 = btn1/stylus, 6 = btn2/eraser)
/// 10       1    (reserved)
/// 11-12    2    Tablet express-key bitmask (active-low, with 0xCC mask on high byte)
/// 13-63        (padding / zeroes)
#[derive(Copy, Clone, Default, Debug)]
pub struct PenData {
    pub x: i32,
    pub y: i32,
    pub pressure_raw: i32,
    pub pen_button: u8,
    pub tablet_buttons: u16,
}

impl PenData {
    #[must_use] 
    pub fn decode(buf: &[u8]) -> Self {
        let x = i32::from(u16_be(buf[1], buf[2]));
        let y = i32::from(u16_be(buf[3], buf[4]));
        let pressure_raw = i32::from(u16_be(buf[5], buf[6]));
        let tablet_buttons = u16_be(buf[12], buf[11]) | (0xCC << 8);
        let pen_button = buf[9];

        Self { x, y, pressure_raw, pen_button, tablet_buttons }
    }
}

fn u16_be(high: u8, low: u8) -> u16 {
    u16::from(high) << 8 | u16::from(low)
}

/// Which tablet express key is pressed, decoded from the active-low bitmask.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TabletKey {
    B1, B2, B3, B4, B5, B6, B7, B8, B9, B10, B11, B12,
}

impl TabletKey {
    pub const ALL: &[TabletKey] = &[
        Self::B1, Self::B2, Self::B3, Self::B4, Self::B5, Self::B6,
        Self::B7, Self::B8, Self::B9, Self::B10, Self::B11, Self::B12,
    ];

    fn bit_index(self) -> u16 {
        match self {
            Self::B1 => 9,
            Self::B2 => 12,
            Self::B3 => 7,
            Self::B4 => 8,
            Self::B5 => 6,
            Self::B6 => 13,
            Self::B7 => 5,
            Self::B8 => 0,
            Self::B9 => 4,
            Self::B10 => 1,
            Self::B11 => 3,
            Self::B12 => 2,
        }
    }

    #[must_use] 
    pub fn is_pressed(self, flags: u16) -> bool {
        (flags & (1 << self.bit_index())) == 0
    }
}

/// Pen barrel button state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PenButton {
    Stylus,  // lower barrel button
    Eraser,  // upper barrel button
}

impl PenButton {
    pub const ALL: &[PenButton] = &[Self::Stylus, Self::Eraser];

    #[must_use] 
    pub fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            4 => Some(Self::Stylus),
            6 => Some(Self::Eraser),
            _ => None,
        }
    }
}
