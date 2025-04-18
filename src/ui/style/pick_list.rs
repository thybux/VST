use nih_plug_iced::PickList;
use nih_plug_iced::{
    pick_list::{Menu, State as PickListState, Style, StyleSheet},
    Color, Element, Length,
};
use std::fmt::Display;

struct CustomStyle;

impl StyleSheet for CustomStyle {
    fn active(&self) -> Style {
        Style {
            text_color: Color::from_rgb(0.9, 0.9, 0.9),
            background: Color::from_rgb(0.1, 0.1, 0.1).into(),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            background: Color::from_rgb(0.2, 0.2, 0.2).into(),
            ..self.active()
        }
    }

    fn menu(&self) -> Menu {
        Menu {
            text_color: Color::from_rgb(0.9, 0.9, 0.9),
            background: Color::from_rgb(0.1, 0.1, 0.1).into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.5, 0.5, 0.5),
            selected_text_color: Color::from_rgb(1.0, 1.0, 1.0),
            selected_background: Color::from_rgb(0.3, 0.3, 0.3).into(),
        }
    }
}

pub fn custom_pick_list<'a, T, F>(
    state: &'a mut PickListState<T>,
    options: &[T],
    selected: Option<T>,
    on_select: F,
) -> Element<'a, T>
where
    T: Display + Clone + Eq + 'static,
    F: 'static + Fn(T) -> T,
{
    let options_owned: Vec<T> = options.to_vec();
    PickList::new(state, options_owned, selected, on_select)
        .style(CustomStyle)
        .width(Length::Fill)
        .into()
}
