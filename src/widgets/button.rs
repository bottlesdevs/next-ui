use iced::{
    Background, Border, Center, Color, Fill, Length,
    theme::palette::Pair,
    widget::{Row, container, svg, text, tooltip},
};

use crate::{
    Element, Theme,
    icons::{self, Icon},
};

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
    /// No fill until hovered, focused or pressed: for header bars, toolbars
    /// and controls set into rows.
    Flat,
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
        let content: Element<'a, Message> = if button.loading {
            container(text("…").label()).center(Fill).into()
        } else if button.shape == Shape::IconOnly {
            container(icon_element(
                button.icon.expect("icon-only buttons always have an icon"),
                button.icon_size,
                button.kind,
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
                row = row.push(icon_element(icon, button.icon_size, button.kind));
            }

            row = row.push(button.content);

            if let Some(icon) = button.icon.filter(|_| button.icon_trailing) {
                row = row.push(icon_element(icon, button.icon_size, button.kind));
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

/// The button's icon, in the colour of its label so both follow its state.
fn icon_element<'a, Message: 'a>(icon: Icon, size: f32, kind: ButtonKind) -> Element<'a, Message> {
    if kind == ButtonKind::Primary {
        icon.view()
            .width(size)
            .height(size)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.colors().muted),
            })
            .into()
    } else {
        icon.inherited(size).into()
    }
}

fn appearance(
    theme: &Theme,
    state: ActionState,
    shape: Shape,
    kind: ButtonKind,
) -> iced::widget::button::Style {
    let colors = colors(theme, state.hovered || state.pressed || state.focused, kind);

    iced::widget::button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(match (shape, kind) {
            (Shape::Rectangular, _) | (Shape::IconOnly, ButtonKind::Flat) => radius::ROW,
            (Shape::Pill | Shape::IconOnly, _) => radius::PILL,
        }),
        ..iced::widget::button::Style::default()
    }
}

/// A kind's resting fill and label. Hover and press come from the state
/// layer every action draws; only a flat button's label brightens too, since
/// it has no fill to carry the emphasis.
fn colors(theme: &Theme, active: bool, kind: ButtonKind) -> Pair {
    let colors = theme.colors();
    let pair = |color, text| Pair { color, text };

    match kind {
        ButtonKind::Primary => pair(colors.chrome, colors.text),
        ButtonKind::Secondary => pair(colors.control, colors.accent_muted),
        ButtonKind::Surface => pair(colors.card, colors.muted),
        ButtonKind::Flat => pair(
            Color::TRANSPARENT,
            if active { colors.text } else { colors.muted },
        ),
        ButtonKind::Accent => pair(colors.accent, colors.chrome),
        ButtonKind::Destructive => pair(colors.danger, colors.text),
    }
}
