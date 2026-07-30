use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    widget::{Row, Space, button as iced_button, container, svg, text},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Shape {
    #[default]
    Rectangular,
    Pill,
    Circular,
}

pub struct Button<'a, Message> {
    label: &'a str,
    icon: Option<svg::Handle>,
    shape: Shape,
    diameter: f32,
    primary: bool,
    on_press: Option<Message>,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            shape: Shape::Rectangular,
            diameter: 52.0,
            primary: false,
            on_press: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<svg::Handle>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn rectangular(mut self) -> Self {
        self.shape = Shape::Rectangular;
        self
    }

    pub fn pill(mut self) -> Self {
        self.shape = Shape::Pill;
        self
    }

    pub fn circular(mut self) -> Self {
        self.shape = Shape::Circular;
        self
    }

    pub fn diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(button: Button<'a, Message>) -> Self {
        let content: Element<'a, Message> = if !shows_label(button.shape) {
            let icon = button
                .icon
                .map(|icon| icon_element(icon, button.primary))
                .unwrap_or_else(|| Space::new().into());

            container(icon).center_x(Fill).center_y(Fill).into()
        } else {
            let mut content = Row::new().spacing(8).align_y(Center);

            if let Some(icon) = button.icon {
                content = content.push(icon_element(icon, button.primary));
            }

            content.push(text(button.label).size(18)).into()
        };
        let shape = button.shape;
        let primary = button.primary;
        let mut widget = iced_button(content)
            .style(move |theme, status| appearance(theme, status, shape, primary));

        widget = match shape {
            Shape::Rectangular => widget.padding([12, 18]),
            Shape::Pill => widget.padding([10, 16]),
            Shape::Circular => widget
                .padding(0)
                .width(Length::Fixed(button.diameter))
                .height(Length::Fixed(button.diameter)),
        };

        if let Some(message) = button.on_press {
            widget = widget.on_press(message);
        }

        widget.into()
    }
}

fn icon_element<'a, Message: 'a>(handle: svg::Handle, primary: bool) -> Element<'a, Message> {
    svg(handle)
        .width(24)
        .height(24)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if primary {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            }),
        })
        .into()
}

fn shows_label(shape: Shape) -> bool {
    shape != Shape::Circular
}

fn appearance(
    theme: &Theme,
    status: iced_button::Status,
    shape: Shape,
    primary: bool,
) -> iced_button::Style {
    let palette = theme.extended_palette();
    let colors = if primary {
        match status {
            iced_button::Status::Hovered | iced_button::Status::Pressed => palette.primary.strong,
            _ => palette.primary.base,
        }
    } else {
        match status {
            iced_button::Status::Hovered | iced_button::Status::Pressed => {
                palette.background.strong
            }
            _ => palette.background.weakest,
        }
    };

    iced_button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(match shape {
            Shape::Rectangular => 8,
            Shape::Pill | Shape::Circular => 999,
        }),
        ..iced_button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Shape, shows_label};

    #[test]
    fn circular_buttons_hide_their_label() {
        assert!(!shows_label(Shape::Circular));
        assert!(shows_label(Shape::Pill));
        assert!(shows_label(Shape::Rectangular));
    }
}
