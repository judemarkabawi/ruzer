use std::str::FromStr;

#[derive(Clone, Debug)]
#[repr(u8)]
pub enum LedId {
    // Zero = 0x00,
    // ScrollWeel = 0x01,
    // Battery = 0x03,
    Logo = 0x04,
    // Backlight = 0x05,
    // Macro = 0x07,
    // Game = 0x08,
}

#[derive(Copy, Clone)]
pub enum BreathingEffect {
    Single(Color),
    Dual(Color, Color),
    Random,
}

#[derive(Copy, Clone)]
pub enum ExtendedMatrixEffect {
    None,
    Static(Color),
    Breathing(BreathingEffect),
    Spectrum,
    // Wave = 0x04,
    /// Reactive effect, color with speed
    Reactive(Color, u8),
    // Starlight = 0x07,
    // Wheel = 0x0A,
}

impl From<ExtendedMatrixEffect> for u8 {
    fn from(value: ExtendedMatrixEffect) -> Self {
        match value {
            ExtendedMatrixEffect::None => 0x00,
            ExtendedMatrixEffect::Static(..) => 0x01,
            ExtendedMatrixEffect::Breathing(..) => 0x02,
            ExtendedMatrixEffect::Spectrum => 0x03,
            ExtendedMatrixEffect::Reactive(..) => 0x05,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl FromStr for Color {
    type Err = ();

    /// Parse a hex code starting with "#" (ex: `#0cff1d`)
    fn from_str(hex_code: &str) -> Result<Self, Self::Err> {
        let r: u8 = u8::from_str_radix(&hex_code[1..3], 16).map_err(|_| ())?;
        let g: u8 = u8::from_str_radix(&hex_code[3..5], 16).map_err(|_| ())?;
        let b: u8 = u8::from_str_radix(&hex_code[5..7], 16).map_err(|_| ())?;
        Ok(Color { r, g, b })
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Self {
            r: value.0,
            g: value.0,
            b: value.0,
        }
    }
}
