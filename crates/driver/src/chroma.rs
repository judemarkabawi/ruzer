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
pub enum EffectBreathing {
    Single(Color),
    Dual(Color, Color),
    Random,
}

#[derive(Copy, Clone)]
pub enum ExtendedMatrixEffect {
    None,
    Static(Color),
    Breathing(EffectBreathing),
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

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Self {
            r: value.0,
            g: value.0,
            b: value.0,
        }
    }
}
