use iced::{
    Background, Border, Center, ContentFit, Element, Fill, Length, Theme,
    theme::palette::Pair,
    widget::{Row, container, svg, text, tooltip},
};

use crate::icons::{self, Icon};

use super::{
    pressable::{Pressable, Status},
    text::TextExt as _,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Shape {
    #[default]
    Rectangular,
    Pill,
    IconOnly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonKind {
    #[default]
    Secondary,
    Primary,
    Surface,
}

pub struct Button<'a, Message> {
    label: &'a str,
    icon: Option<Icon>,
    shape: Shape,
    diameter: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
    loading: bool,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            shape: Shape::Rectangular,
            diameter: 52.0,
            kind: ButtonKind::Secondary,
            on_press: None,
            loading: false,
        }
    }

    pub fn icon_only(label: &'a str, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            shape: Shape::IconOnly,
            ..Self::new(label)
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
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

    pub fn diameter(mut self, diameter: f32) -> Self {
        if self.shape == Shape::IconOnly {
            self.diameter = diameter.max(1.0);
        }

        self
    }

    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn primary(self) -> Self {
        self.kind(ButtonKind::Primary)
    }

    pub fn surface(self) -> Self {
        self.kind(ButtonKind::Surface)
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(button: Button<'a, Message>) -> Self {
        let disabled = button.loading || button.on_press.is_none();
        let content: Element<'a, Message> = if button.loading {
            container(text("…").label()).center(Fill).into()
        } else if button.shape == Shape::IconOnly {
            container(icon_element(
                button.icon.expect("icon-only buttons always have an icon"),
                button.kind,
                disabled,
            ))
            .center(Fill)
            .into()
        } else {
            let mut content = Row::new().spacing(8).align_y(Center);

            if let Some(icon) = button.icon {
                content = content.push(icon_element(icon, button.kind, disabled));
            }

            content.push(text(button.label).label()).into()
        };
        let shape = button.shape;
        let kind = button.kind;
        let mut pressable = Pressable::new(content)
            .on_press_maybe((!button.loading).then_some(button.on_press).flatten())
            .style(move |theme, status| appearance(theme, status, shape, kind));

        pressable = match shape {
            Shape::Rectangular => pressable.padding([12, 18]),
            Shape::Pill => pressable.padding([10, 16]),
            Shape::IconOnly => pressable
                .width(Length::Fixed(button.diameter))
                .height(Length::Fixed(button.diameter)),
        };

        let element: Element<'a, Message> = pressable.into();

        if shape == Shape::IconOnly {
            tooltip(element, text(button.label), tooltip::Position::Bottom)
                .gap(6)
                .into()
        } else {
            element
        }
    }
}

fn icon_element<'a, Message: 'a>(
    icon: Icon,
    kind: ButtonKind,
    disabled: bool,
) -> Element<'a, Message> {
    svg(icon.handle())
        .width(icons::SIZE)
        .height(icons::SIZE)
        .content_fit(ContentFit::Contain)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if disabled {
                theme.extended_palette().secondary.weak.text
            } else {
                colors(theme, Status::Active, kind).text
            }),
        })
        .into()
}

fn appearance(
    theme: &Theme,
    status: Status,
    shape: Shape,
    kind: ButtonKind,
) -> iced::widget::button::Style {
    let colors = colors(theme, status, kind);

    iced::widget::button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(match shape {
            Shape::Rectangular => 8,
            Shape::Pill | Shape::IconOnly => 999,
        }),
        ..iced::widget::button::Style::default()
    }
}

fn colors(theme: &Theme, status: Status, kind: ButtonKind) -> Pair {
    let palette = theme.extended_palette();

    if status == Status::Disabled {
        return Pair {
            color: palette.background.weaker.color,
            text: palette.secondary.weak.text,
        };
    }

    match kind {
        ButtonKind::Primary => match status {
            Status::Hovered | Status::Focused => palette.primary.strong,
            Status::Pressed => palette.primary.weak,
            _ => palette.primary.base,
        },
        ButtonKind::Secondary => match status {
            Status::Hovered | Status::Focused => palette.secondary.strong,
            Status::Pressed => palette.secondary.weak,
            _ => palette.secondary.base,
        },
        ButtonKind::Surface => {
            let background = match status {
                Status::Hovered | Status::Focused => palette.background.strong,
                Status::Pressed => palette.background.stronger,
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
    use super::{Button, ButtonKind, Shape, Status, appearance};
    use crate::{icons::Icon, theme};

    #[test]
    fn icon_only_constructor_always_has_an_icon() {
        let button = Button::<()>::icon_only("Play", Icon::Play);

        assert_eq!(button.shape, Shape::IconOnly);
        assert_eq!(button.icon, Some(Icon::Play));
    }

    #[test]
    fn disabled_buttons_are_visually_distinct() {
        let theme = theme::theme();

        assert_ne!(
            appearance(
                &theme,
                Status::Active,
                Shape::Rectangular,
                ButtonKind::Primary,
            ),
            appearance(
                &theme,
                Status::Disabled,
                Shape::Rectangular,
                ButtonKind::Primary,
            ),
        );
    }

    #[test]
    fn loading_disables_activation() {
        let button = Button::new("Install").on_press(()).loading(true);

        assert!(button.loading);
        assert!(button.on_press.is_some());
    }
}
