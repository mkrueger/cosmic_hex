use cosmic::iced::{Color, Size};

#[derive(Default)]
pub struct Theme {
    pub caret: Color,
    pub background: Color,
    pub offset_number: Color,
    pub hex: Color,
    pub ascii: Color,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            caret: Color::BLACK,
            background: Color::WHITE,
            offset_number: Color::from_rgb8(155, 90, 90),
            hex: Color::from_rgb8(90, 90, 90),
            ascii: Color::from_rgb8(90, 90, 90),
        }
    }

    pub(crate) fn calc_cell_width(&self, font_measure: Size<f32>) -> f32 {
        font_measure.width * 2.0 + 5.0
    }

    pub(crate) fn calc_offset_margin_width(&self, font_measure: Size<f32>) -> f32 {
        font_measure.width * 8.0 + 10.0
    }

    pub(crate) fn hex_ascii_spacing(&self) -> f32 {
        5.0
    }
}
