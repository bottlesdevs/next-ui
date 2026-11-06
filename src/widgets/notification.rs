//! A dismissible notification card with a copy action.

use iced::{
    Center, Element, Fill, Theme,
    widget::{column, opaque, row, svg, text},
};

use super::{
    button::{Button, ButtonKind},
    card::Card,
    info_card::{self, Kind},
    spacing,
    text::TextExt as _,
};
use crate::icons::Icon;

/// A notification whose content and actions are owned by the caller.
pub struct Notification<'a, Message> {
    kind: Kind,
    title: &'a str,
    message: &'a str,
    on_dismiss: Option<Message>,
    on_copy: Option<Message>,
}

impl<'a, Message> Notification<'a, Message> {
    /// Creates a card with the kind's semantic icon and colour.
    pub fn new(kind: Kind, title: &'a str, message: &'a str) -> Self {
        Self {
            kind,
            title,
            message,
            on_dismiss: None,
            on_copy: None,
        }
    }

    /// Enables the dismiss button.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Enables the copy button.
    pub fn on_copy(mut self, message: Message) -> Self {
        self.on_copy = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Notification<'a, Message>> for Element<'a, Message> {
    fn from(notification: Notification<'a, Message>) -> Self {
        let kind = notification.kind;
        let dismiss = Button::icon_only("Dismiss notification", Icon::Cross)
            .diameter(28.0)
            .icon_size(14.0)
            .kind(ButtonKind::Transparent)
            .on_press_maybe(notification.on_dismiss);
        let copy = Button::new("Copy")
            .kind(ButtonKind::Transparent)
            .on_press_maybe(notification.on_copy);
        let header = row![
            info_card::icon(kind).style(move |theme: &Theme, _| svg::Style {
                color: Some(info_card::colors(theme, kind).color),
            }),
            text(notification.title).size(17).medium().width(Fill),
            dismiss,
        ]
        .spacing(8)
        .align_y(Center);
        opaque(
            Card::new(
                column![header, text(notification.message).size(14), copy].spacing(spacing::SM),
            )
            .width(Fill)
            .padding(16),
        )
    }
}
