use nih_plug::prelude::ParamPtr;
use nih_plug_iced::alignment;
use nih_plug_iced::backend::Renderer;
use nih_plug_iced::renderer::Renderer as GraphicsRenderer;
use nih_plug_iced::text::Renderer as TextRenderer;
use nih_plug_iced::widgets::ParamMessage;
use nih_plug_iced::{
    event, layout, mouse, renderer, Color, Element, Event, Layout, Length, Point, Rectangle, Size,
    Widget,
};

// Constantes
const BORDER_WIDTH: f32 = 1.0;
const INDICATOR_WIDTH: f32 = 2.0;

/// Un potentiomètre simple pour nih-plug
pub struct ParamKnob<'a> {
    param_ptr: ParamPtr,
    size: u16,
    text_size: u16,
    show_value: bool,
    label: Option<String>,
    // État interne
    is_dragging: &'a mut bool,
    last_y: &'a mut f32,
}

impl<'a> ParamKnob<'a> {
    /// Crée un nouveau potentiomètre
    pub fn new(param_ptr: ParamPtr, is_dragging: &'a mut bool, last_y: &'a mut f32) -> Self {
        Self {
            param_ptr,
            size: 40,
            text_size: 13,
            show_value: true,
            label: None,
            is_dragging,
            last_y,
        }
    }

    /// Définit la taille
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    /// Définit la taille du texte
    pub fn text_size(mut self, size: u16) -> Self {
        self.text_size = size;
        self
    }

    /// Affiche ou masque la valeur
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Ajoute un label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Convertit ce paramètre en élément avec un message personnalisé
    pub fn map<Message, F>(self, f: F) -> Element<'a, Message>
    where
        Message: 'static,
        F: Fn(ParamMessage) -> Message + 'static,
    {
        Element::from(self).map(f)
    }
}

impl<'a> Widget<ParamMessage, Renderer> for ParamKnob<'a> {
    fn width(&self) -> Length {
        Length::Units(self.size)
    }

    fn height(&self) -> Length {
        let base = self.size;
        let extra = if self.label.is_some() || self.show_value {
            self.text_size + 4
        } else {
            0
        };
        Length::Units(base + extra)
    }

    fn layout(&self, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width()).height(self.height());
        let size = limits.resolve(Size::ZERO);
        layout::Node::new(size)
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn nih_plug_iced::Clipboard,
        shell: &mut nih_plug_iced::Shell<'_, ParamMessage>,
    ) -> event::Status {
        let knob_bounds = Rectangle {
            x: layout.bounds().x,
            y: layout.bounds().y,
            width: self.size as f32,
            height: self.size as f32,
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if knob_bounds.contains(cursor_position) {
                    println!("Knob clicked at y={}", cursor_position.y);
                    *self.is_dragging = true;
                    *self.last_y = cursor_position.y;

                    shell.publish(ParamMessage::BeginSetParameter(self.param_ptr));
                    return event::Status::Captured;
                }
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if *self.is_dragging {
                    println!("Drag released");
                    *self.is_dragging = false;
                    shell.publish(ParamMessage::EndSetParameter(self.param_ptr));
                    return event::Status::Captured;
                }
            }

            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if *self.is_dragging {
                    let current_y = position.y;
                    let delta_y = *self.last_y - current_y;
                    *self.last_y = current_y;

                    // Récupérer la valeur actuelle
                    let current_value = unsafe { self.param_ptr.modulated_normalized_value() };

                    // Calculer la nouvelle valeur (vers le haut = augmente)
                    let delta_normalized = delta_y / 100.0;
                    let new_value = (current_value + delta_normalized).clamp(0.0, 1.0);

                    println!(
                        "Dragging: delta_y={}, current={}, new={}",
                        delta_y, current_value, new_value
                    );

                    // Publier la nouvelle valeur
                    shell.publish(ParamMessage::SetParameterNormalized(
                        self.param_ptr,
                        new_value,
                    ));

                    return event::Status::Captured;
                }
            }

            _ => {}
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let knob_bounds = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: self.size as f32,
            height: self.size as f32,
        };

        // Dessiner le fond du potentiomètre
        let is_hovered = knob_bounds.contains(cursor_position);
        let background_color = if is_hovered || *self.is_dragging {
            Color::new(0.3, 0.3, 0.3, 1.0)
        } else {
            Color::new(0.2, 0.2, 0.2, 1.0)
        };

        let center_x = knob_bounds.x + knob_bounds.width / 2.0;
        let center_y = knob_bounds.y + knob_bounds.height / 2.0;
        let radius = knob_bounds.width / 2.0 - BORDER_WIDTH;

        // Cercle de fond
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: center_x - radius,
                    y: center_y - radius,
                    width: radius * 2.0,
                    height: radius * 2.0,
                },
                border_radius: radius,
                border_width: BORDER_WIDTH,
                border_color: Color::BLACK,
            },
            background_color,
        );

        // Récupérer la valeur actuelle
        let current_value = unsafe { self.param_ptr.modulated_normalized_value() };

        // Dessiner l'indicateur
        let start_angle = -2.35; // Environ -135 degrés
        let angle = start_angle + (4.7 * current_value); // Faire 270 degrés

        let indicator_length = radius - 4.0;
        let end_x = center_x + angle.cos() * indicator_length;
        let end_y = center_y + angle.sin() * indicator_length;

        // Dessiner l'indicateur comme une série de points pour simuler une ligne
        let num_segments = 10;
        for i in 0..=num_segments {
            let t = i as f32 / num_segments as f32;
            let seg_x = center_x + (end_x - center_x) * t;
            let seg_y = center_y + (end_y - center_y) * t;

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: seg_x - (INDICATOR_WIDTH / 2.0),
                        y: seg_y - (INDICATOR_WIDTH / 2.0),
                        width: INDICATOR_WIDTH,
                        height: INDICATOR_WIDTH,
                    },
                    border_radius: INDICATOR_WIDTH / 2.0,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
                Color::WHITE,
            );
        }

        // Dessiner le point central
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: center_x - 2.0,
                    y: center_y - 2.0,
                    width: 4.0,
                    height: 4.0,
                },
                border_radius: 2.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color::WHITE,
        );

        // Afficher la valeur
        if self.show_value {
            let value_y = knob_bounds.y + knob_bounds.height + 2.0;

            let value_text = unsafe {
                self.param_ptr
                    .normalized_value_to_string(current_value, true)
            };

            renderer.fill_text(nih_plug_iced::text::Text {
                content: &value_text,
                size: self.text_size as f32,
                bounds: Rectangle {
                    x: center_x,
                    y: value_y,
                    width: knob_bounds.width,
                    height: self.text_size as f32 + 4.0,
                },
                color: style.text_color,
                horizontal_alignment: alignment::Horizontal::Center,
                vertical_alignment: alignment::Vertical::Center,
                font: <Renderer as TextRenderer>::Font::default(),
            });
        }

        // Afficher le label
        if let Some(label) = &self.label {
            let label_y = if self.show_value {
                knob_bounds.y + knob_bounds.height + self.text_size as f32 + 6.0
            } else {
                knob_bounds.y + knob_bounds.height + 2.0
            };

            renderer.fill_text(nih_plug_iced::text::Text {
                content: label,
                size: self.text_size as f32,
                bounds: Rectangle {
                    x: center_x,
                    y: label_y,
                    width: knob_bounds.width,
                    height: self.text_size as f32 + 4.0,
                },
                color: style.text_color,
                horizontal_alignment: alignment::Horizontal::Center,
                vertical_alignment: alignment::Vertical::Center,
                font: <Renderer as TextRenderer>::Font::default(),
            });
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let knob_bounds = Rectangle {
            x: layout.bounds().x,
            y: layout.bounds().y,
            width: self.size as f32,
            height: self.size as f32,
        };

        if knob_bounds.contains(cursor_position) || *self.is_dragging {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a> From<ParamKnob<'a>> for Element<'a, ParamMessage> {
    fn from(knob: ParamKnob<'a>) -> Self {
        Element::new(knob)
    }
}
