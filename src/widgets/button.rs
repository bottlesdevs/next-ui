use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    theme::palette::Pair,
    widget::{Row, container, svg, text, tooltip},
};

use crate::icons::{self, Icon};

use super::{
    focusable_action::{ActionState, FocusableAction},
    radius, spacing,
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
    /// The light accent fill for a page's main action, e.g. Install.
    Accent,
    /// The danger fill for confirming a destructive action, e.g. Delete.
    Destructive,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Regular,
    /// Tighter padding, for a button set into a dense list such as a
    /// picker's options.
    Compact,
}

pub struct Button<'a, Message> {
    content: Element<'a, Message>,
    tooltip: Option<&'a str>,
    icon: Option<Icon>,
    icon_trailing: bool,
    icon_size: f32,
    shape: Shape,
    diameter: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
    loading: bool,
    size: ButtonSize,
}

impl<'a, Message> Button<'a, Message> {
    pub fn new(label: impl text::IntoFragment<'a>) -> Self {
        Self::custom(text(label))
    }

    pub fn custom(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            tooltip: None,
            icon: None,
            icon_trailing: false,
            icon_size: icons::SIZE,
            shape: Shape::Rectangular,
            diameter: 52.0,
            kind: ButtonKind::Secondary,
            on_press: None,
            loading: false,
            size: ButtonSize::Regular,
        }
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn compact(self) -> Self {
        self.size(ButtonSize::Compact)
    }

    pub fn icon_only(tooltip: &'a str, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            shape: Shape::IconOnly,
            tooltip: Some(tooltip),
            ..Self::custom(text(""))
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
                row = row.push(icon_element(icon, button.icon_size, button.kind, disabled));
            }

            row = row.push(button.content);

            if let Some(icon) = button.icon.filter(|_| button.icon_trailing) {
                row = row.push(icon_element(icon, button.icon_size, button.kind, disabled));
            }

            row.into()
        };

        let shape = button.shape;
        let kind = button.kind;
        let mut control = FocusableAction::new(content)
            .on_press_maybe(button.on_press.filter(|_| !button.loading))
            .style(move |theme, status| appearance(theme, status, shape, kind));

        control = match shape {
            Shape::Rectangular | Shape::Pill if button.size == ButtonSize::Compact => {
                control.padding([8.0, 11.0])
            }
            Shape::Rectangular | Shape::Pill => control.padding(if kind == ButtonKind::Primary {
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
            Shape::IconOnly => control
                .width(Length::Fixed(button.diameter))
                .height(Length::Fixed(button.diameter)),
        };

        let element: Element<'a, Message> = control.into();

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
    size: f32,
    kind: ButtonKind,
    disabled: bool,
) -> Element<'a, Message> {
    icon.view()
        .width(size)
        .height(size)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if disabled || kind == ButtonKind::Primary {
                crate::theme::colors(theme).muted
            } else {
                colors(theme, false, false, false, kind).text
            }),
        })
        .into()
}

fn appearance(
    theme: &Theme,
    state: ActionState,
    shape: Shape,
    kind: ButtonKind,
) -> iced::widget::button::Style {
    let semantic = crate::theme::colors(theme);
    let colors = colors(
        theme,
        !state.enabled,
        state.hovered || state.focused,
        state.pressed,
        kind,
    );

    iced::widget::button::Style {
        background: if kind == ButtonKind::Transparent {
            if state.focused && !state.hovered {
                Some(Background::Color(semantic.hovered))
            } else {
                None
            }
        } else {
            Some(Background::Color(colors.color))
        },
        text_color: colors.text,
        border: Border::default().rounded(match (shape, kind) {
            (Shape::Rectangular, _) | (Shape::IconOnly, ButtonKind::Transparent) => radius::ROW,
            (Shape::Pill | Shape::IconOnly, _) => radius::PILL,
        }),
        ..iced::widget::button::Style::default()
    }
}

fn colors(theme: &Theme, disabled: bool, active: bool, pressed: bool, kind: ButtonKind) -> Pair {
    let colors = crate::theme::colors(theme);

    if disabled {
        return Pair {
            color: colors.page,
            text: colors.muted,
        };
    }

    match kind {
        ButtonKind::Primary => {
            if pressed {
                Pair {
                    color: colors.sunken,
                    text: colors.text,
                }
            } else {
                Pair {
                    color: if active {
                        colors.chrome_hovered
                    } else {
                        colors.chrome
                    },
                    text: colors.text,
                }
            }
        }
        ButtonKind::Secondary => Pair {
            color: if pressed {
                colors.chrome
            } else if active {
                colors.page
            } else {
                colors.control
            },
            text: colors.accent_muted,
        },
        ButtonKind::Surface => {
            let background = if pressed {
                colors.selected
            } else if active {
                colors.hovered
            } else {
                colors.card
            };

            Pair {
                color: background,
                text: colors.muted,
            }
        }
        // No fill, so the pointer shows in the label, as it does on tabs.
        ButtonKind::Transparent => Pair {
            color: colors.page,
            text: if active || pressed {
                colors.text
            } else {
                colors.muted
            },
        },
        ButtonKind::Accent => Pair {
            color: if pressed || active {
                colors.accent_muted
            } else {
                colors.accent
            },
            text: colors.chrome,
        },
        ButtonKind::Destructive => Pair {
            color: if pressed || active {
                colors.danger.scale_alpha(0.8)
            } else {
                colors.danger
            },
            text: colors.text,
        },
    }
}
