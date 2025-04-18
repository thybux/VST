use nih_plug_iced::{container, Color};

pub struct MainPage;

impl container::StyleSheet for MainPage {
    fn style(&self) -> container::Style {
        container::Style {
            background: Color::from_rgb8(0x28, 0x2C, 0x34).into(),
            text_color: Color::WHITE.into(),
            // border_radius: 10.0,
            // border_width: 1.0,
            // border_color: Color::from_rgb8(200, 200, 200),
            ..container::Style::default()
        }
    }
}

