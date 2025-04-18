use nih_plug_iced::{button, Color};

pub struct GenerateButton;

impl button::StyleSheet for GenerateButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: Color::from_rgb8(0x28, 0x2C, 0x34).into(),
            border_radius: 8.0,
            border_width: 0.0,
            shadow_offset: nih_plug_iced::Vector::new(0.0, 0.0),
            border_color: Default::default(),
            text_color: Color::from_rgb8(0xDF, 0xE1, 0xE5).into(),
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Color::from_rgb8(0x3A, 0x3F, 0x46).into(),
            border_radius: 8.0,
            border_width: 0.0,
            shadow_offset: nih_plug_iced::Vector::new(0.0, 0.0),
            border_color: Default::default(),
            text_color: Color::WHITE,
        }
    }
}

