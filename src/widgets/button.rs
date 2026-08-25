use iced::{
    Background, Border, Center, ContentFit, Element, Fill, Length, Theme,
    theme::palette::Pair,
    widget::{Row, container, svg, text, tooltip},
};

use crate::icons::{self, Icon};

use super::{
    pressable::{Pressable, Status},
    spacing,
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
    Transparent,
}

pub struct Button<'a, Message> {
    content: Element<'a, Message>,
    tooltip: Option<&'a str>,
    icon: Option<Icon>,
    icon_trailing: bool,
    icon_rotation: f32,
    icon_size: f32,
    shape: Shape,
    diameter: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
    loading: bool,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            tooltip: None,
            icon: None,
            icon_trailing: false,
            icon_rotation: 0.0,
            icon_size: icons::SIZE,
            shape: Shape::Rectangular,
            diameter: 52.0,
            kind: ButtonKind::Secondary,
            on_press: None,
            loading: false,
        }
    }

    pub fn icon_only(tooltip: &'a str, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            shape: Shape::IconOnly,
            tooltip: Some(tooltip),
            ..Self::new(text(""))
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.icon_trailing = false;
        self
    }

    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.icon_trailing = true;
        self
    }

    pub fn icon_rotation(mut self, rotation: f32) -> Self {
        self.icon_rotation = rotation;
        self
    }

    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size.max(1.0);
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
                button.icon_rotation,
                button.icon_size,
                button.kind,
                disabled,
            ))
            .center(Fill)
            .into()
        } else {
            let mut row = Row::new()
                .spacing(if button.kind == ButtonKind::Primary {
                    16.0
                } else {
                    spacing::XS
                })
                .align_y(Center);

            if let Some(icon) = button.icon.filter(|_| !button.icon_trailing) {
                row = row.push(icon_element(
                    icon,
                    button.icon_rotation,
                    button.icon_size,
                    button.kind,
                    disabled,
                ));
            }

            row = row.push(button.content);

            if let Some(icon) = button.icon.filter(|_| button.icon_trailing) {
                row = row.push(icon_element(
                    icon,
                    button.icon_rotation,
                    button.icon_size,
                    button.kind,
                    disabled,
                ));
            }

            row.into()
        };

        let shape = button.shape;
        let kind = button.kind;
        let mut pressable = Pressable::new(content)
            .on_press_maybe(button.on_press.filter(|_| !button.loading))
            .style(move |theme, status| appearance(theme, status, shape, kind));

        pressable = match shape {
            Shape::Rectangular | Shape::Pill => pressable.padding(if kind == ButtonKind::Primary {
                [spacing::MD, spacing::XG]
            } else {
                [
                    if kind == ButtonKind::Surface {
                        spacing::XS
                    } else {
                        spacing::SM
                    },
                    spacing::MD,
                ]
            }),
            Shape::IconOnly => pressable
                .width(Length::Fixed(button.diameter))
                .height(Length::Fixed(button.diameter)),
        };

        let element: Element<'a, Message> = pressable.into();

        if shape == Shape::IconOnly {
            if let Some(tooltip_text) = button.tooltip {
                tooltip(element, text(tooltip_text), tooltip::Position::Bottom)
                    .gap(spacing::XS)
                    .into()
            } else {
                element
            }
        } else {
            element
        }
    }
}

fn icon_element<'a, Message: 'a>(
    icon: Icon,
    rotation: f32,
    size: f32,
    kind: ButtonKind,
    disabled: bool,
) -> Element<'a, Message> {
    svg(icon.handle())
        .width(size)
        .height(size)
        .content_fit(ContentFit::Contain)
        .rotation(rotation)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if disabled {
                theme.extended_palette().secondary.weak.text
            } else if kind == ButtonKind::Primary {
                theme.extended_palette().primary.base.color
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
        background: if kind == ButtonKind::Transparent {
            match status {
                Status::Pressed => Some(Background::Color(
                    theme.extended_palette().background.stronger.color,
                )),
                Status::Focused => Some(Background::Color(
                    theme.extended_palette().background.strong.color,
                )),
                _ => None,
            }
        } else {
            Some(Background::Color(colors.color))
        },
        text_color: colors.text,
        border: Border::default().rounded(match (shape, kind) {
            (Shape::Rectangular, _) | (Shape::IconOnly, ButtonKind::Transparent) => 6.0,
            (Shape::Pill | Shape::IconOnly, _) => 999.0,
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
            Status::Hovered | Status::Focused => palette.background.strongest,
            Status::Pressed => palette.background.weakest,
            _ => crate::theme::BottlesTheme::from(theme).hint,
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
        ButtonKind::Transparent => Pair {
            color: palette.background.base.color,
            text: palette.secondary.weak.text,
        },
    }
}
