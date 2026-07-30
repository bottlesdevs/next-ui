use iced::{
    Border, Element, Fill, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, row, text},
};

use crate::icons;

use super::style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleKind {
    Gaming,
    Software,
    Custom,
}

impl BottleKind {
    fn icon(self) -> &'static str {
        match self {
            Self::Gaming => "controller",
            Self::Software => "hollow-gear",
            Self::Custom => "custom",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Gaming => "Gaming",
            Self::Software => "Software",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone)]
pub enum BottleEntryStatus<Message> {
    Ready(Message),
    Progress(u8),
}

pub struct BottleEntry<'a, Message> {
    name: &'a str,
    kind: BottleKind,
    status: BottleEntryStatus<Message>,
}

impl<'a, Message> BottleEntry<'a, Message> {
    pub fn new(name: &'a str, kind: BottleKind, status: BottleEntryStatus<Message>) -> Self {
        Self { name, kind, status }
    }
}

impl<'a, Message: Clone + 'a> From<BottleEntry<'a, Message>> for Element<'a, Message> {
    fn from(entry: BottleEntry<'a, Message>) -> Self {
        let BottleEntry { name, kind, status } = entry;

        let labels = column![
            text(name).size(36).style(text::base),
            row![
                icons::view(kind.icon()),
                text(kind.label()).size(34).style(muted),
            ]
            .spacing(16)
            .align_y(Vertical::Center),
        ]
        .spacing(4);

        let (trailing, on_press): (Element<'a, Message>, Option<Message>) = match status {
            BottleEntryStatus::Ready(message) => {
                (icons::rotated("arrow", std::f32::consts::PI), Some(message))
            }
            BottleEntryStatus::Progress(progress) => (
                container(
                    column![
                        text(progress.min(100)).size(26).style(text::base),
                        text("%").size(26).style(text::base),
                    ]
                    .spacing(0)
                    .align_x(Horizontal::Center),
                )
                .center(80)
                .style(|theme| {
                    let mut appearance = style::surface(theme);
                    appearance.border = Border::default()
                        .color(theme.extended_palette().secondary.weak.text)
                        .width(4)
                        .rounded(40);
                    appearance
                })
                .into(),
                None,
            ),
        };

        let entry =
            button(row![labels, Space::new().width(Fill), trailing].align_y(Vertical::Center))
                .padding([28, 44])
                .width(Fill)
                .style(style::action);

        match on_press {
            Some(message) => entry.on_press(message).into(),
            None => entry.into(),
        }
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
