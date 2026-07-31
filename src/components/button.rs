use iced::{
    Background, Border, Center, ContentFit, Element, Fill, Length, Theme,
    theme::palette::Pair,
    widget::{Row, Space, button as iced_button, container, svg, text},
};

use super::text::TextExt as _;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Shape {
    #[default]
    Rectangular,
    Pill,
    Circular,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Tone {
    #[default]
    Secondary,
    Primary,
    Surface,
}

pub struct Button<'a, Message> {
    label: &'a str,
    icon: Option<svg::Handle>,
    shape: Shape,
    diameter: f32,
    tone: Tone,
    on_press: Option<Message>,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            shape: Shape::Rectangular,
            diameter: 52.0,
            tone: Tone::Secondary,
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
        self.tone = Tone::Primary;
        self
    }

    pub fn surface(mut self) -> Self {
        self.tone = Tone::Surface;
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
                .map(|icon| icon_element(icon, button.tone))
                .unwrap_or_else(|| Space::new().into());

            container(icon).center_x(Fill).center_y(Fill).into()
        } else {
            let mut content = Row::new().spacing(8).align_y(Center);

            if let Some(icon) = button.icon {
                content = content.push(icon_element(icon, button.tone));
            }

            content.push(text(button.label).label()).into()
        };
        let shape = button.shape;
        let tone = button.tone;
        let mut widget =
            iced_button(content).style(move |theme, status| appearance(theme, status, shape, tone));

        widget = match shape {
            Shape::Rectangular => widget.padding([12, 18]),
            Shape::Pill => widget.padding([10, 16]),
            Shape::Circular => widget
                .padding(0)
                .width(Length::Fixed(button.diameter))
                .height(Length::Fixed(button.diameter)),
        };

        widget.on_press_maybe(button.on_press).into()
    }
}

fn icon_element<'a, Message: 'a>(handle: svg::Handle, tone: Tone) -> Element<'a, Message> {
    svg(handle)
        .width(crate::icons::SIZE)
        .height(crate::icons::SIZE)
        .content_fit(ContentFit::Contain)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(colors(theme, iced_button::Status::Active, tone).text),
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
    tone: Tone,
) -> iced_button::Style {
    let colors = colors(theme, status, tone);

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

fn colors(theme: &Theme, status: iced_button::Status, tone: Tone) -> Pair {
    let palette = theme.extended_palette();

    match tone {
        Tone::Primary => match status {
            iced_button::Status::Hovered => palette.primary.strong,
            iced_button::Status::Pressed => palette.primary.weak,
            _ => palette.primary.base,
        },
        Tone::Secondary => match status {
            iced_button::Status::Hovered => palette.secondary.strong,
            iced_button::Status::Pressed => palette.secondary.weak,
            _ => palette.secondary.base,
        },
        Tone::Surface => {
            let background = match status {
                iced_button::Status::Hovered => palette.background.strong,
                iced_button::Status::Pressed => palette.background.stronger,
                _ => palette.background.weak,
            };

            Pair {
                color: background.color,
                text: palette.secondary.base.text,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Shape, Tone, appearance, colors, shows_label};
    use crate::theme;
    use iced::{Background, widget::button};

    #[test]
    fn circular_buttons_hide_their_label() {
        assert!(!shows_label(Shape::Circular));
        assert!(shows_label(Shape::Pill));
        assert!(shows_label(Shape::Rectangular));
    }

    #[test]
    fn pressing_changes_the_button_color() {
        let theme = theme::theme();

        for tone in [Tone::Primary, Tone::Secondary, Tone::Surface] {
            assert_ne!(
                appearance(&theme, button::Status::Hovered, Shape::Circular, tone).background,
                appearance(&theme, button::Status::Pressed, Shape::Circular, tone).background,
            );
        }
    }

    #[test]
    fn circular_buttons_use_the_card_action_colors() {
        let theme = theme::theme();
        let primary = appearance(
            &theme,
            button::Status::Active,
            Shape::Circular,
            Tone::Primary,
        );
        let secondary = appearance(
            &theme,
            button::Status::Active,
            Shape::Circular,
            Tone::Secondary,
        );
        let surface = colors(&theme, button::Status::Active, Tone::Surface);

        assert_eq!(primary.background, Some(Background::Color(theme::MUTED)));
        assert_eq!(primary.text_color, theme::DEEP_BACKGROUND);
        assert_eq!(
            secondary.background,
            Some(Background::Color(theme::DEEP_BACKGROUND))
        );
        assert_eq!(secondary.text_color, theme::MUTED);
        assert_eq!(surface.color, theme::SURFACE);
        assert_eq!(surface.text, theme::MUTED);
    }
}
