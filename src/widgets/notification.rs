//! A dismissible notification in its kind's colour, with optional copy and
//! action buttons.

use iced::{
    Alignment, Border, Center, Element, Fill, Theme,
    widget::{Row, column, container, opaque, row, text},
};

use super::{
    button::{Button, ButtonKind},
    info_card::{self, Kind},
    radius, spacing,
    text::TextExt as _,
    text_size,
};
use crate::icons::Icon;

/// A notification whose content and actions are owned by the caller.
pub struct Notification<'a, Message> {
    kind: Kind,
    title: &'a str,
    message: &'a str,
    on_dismiss: Option<Message>,
    on_copy: Option<Message>,
    action: Option<(&'a str, Message)>,
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
            action: None,
        }
    }

    /// Enables the dismiss button.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Shows a copy button next to dismiss, e.g. for error details.
    pub fn on_copy(mut self, message: Message) -> Self {
        self.on_copy = Some(message);
        self
    }

    /// Adds one action under the message, e.g. "Retry".
    pub fn action(mut self, label: &'a str, message: Message) -> Self {
        self.action = Some((label, message));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Notification<'a, Message>> for Element<'a, Message> {
    fn from(notification: Notification<'a, Message>) -> Self {
        let kind = notification.kind;
        let control = |tooltip, icon| {
            Button::icon_only(tooltip, icon)
                .diameter(32.0)
                .icon_size(16.0)
                .kind(ButtonKind::Transparent)
        };

        let mut controls = Row::new().spacing(2);
        if let Some(on_copy) = notification.on_copy {
            controls = controls.push(control("Copy", Icon::Copy).on_press(on_copy));
        }
        controls = controls.push(
            control("Dismiss notification", Icon::Cross).on_press_maybe(notification.on_dismiss),
        );

        let header = row![
            info_card::icon(kind),
            text(notification.title)
                .size(text_size::HEADING)
                .medium()
                .width(Fill),
            controls,
        ]
        .spacing(10)
        .align_y(Center);

        let mut body =
            column![header, text(notification.message).size(text_size::DENSE)].spacing(spacing::XS);
        if let Some((label, message)) = notification.action {
            body = body.push(
                container(
                    Button::new(label)
                        .kind(ButtonKind::Surface)
                        .on_press(message),
                )
                .padding([spacing::XS, 0.0]),
            );
        }

        opaque(
            container(body)
                .width(Fill)
                .padding([14, 12])
                .align_y(Alignment::Start)
                .style(move |theme: &Theme| {
                    let colors = info_card::colors(theme, kind);
                    container::Style::default()
                        .background(colors.color)
                        .color(colors.text)
                        .border(Border::default().rounded(radius::BOX))
                }),
        )
    }
}
