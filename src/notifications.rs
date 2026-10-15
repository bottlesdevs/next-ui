//! Session notifications and their overlay layer.

use iced::{
    Element, Fill,
    widget::{column, container, responsive, scrollable},
};
use uuid::Uuid;

use crate::{
    app::Message,
    widget::{InfoCardKind, Notification as NotificationCard},
};

pub(crate) struct Notification {
    pub(crate) id: Uuid,
    pub(crate) kind: InfoCardKind,
    pub(crate) title: String,
    pub(crate) message: String,
}

impl Notification {
    pub(crate) fn new(
        kind: InfoCardKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            title: title.into(),
            message: message.into(),
        }
    }
}

#[derive(Default)]
pub(crate) struct Notifications(Vec<Notification>);

impl Notifications {
    pub(crate) fn info(&mut self, message: impl Into<String>) {
        self.push(Notification::new(
            InfoCardKind::Info,
            "Information",
            message,
        ));
    }

    pub(crate) fn error(&mut self, message: impl Into<String>) {
        self.push(Notification::new(InfoCardKind::Error, "Error", message));
    }

    pub(crate) fn push(&mut self, notification: Notification) {
        self.0.push(notification);
    }

    pub(crate) fn dismiss(&mut self, id: Uuid) {
        self.0.retain(|notification| notification.id != id);
    }

    pub(crate) fn get(&self, id: Uuid) -> Option<&Notification> {
        self.0.iter().find(|notification| notification.id == id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.0.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(crate) fn layer(notifications: &Notifications) -> Element<'_, Message> {
    responsive(move |size| {
        let cards = column(notifications.iter().map(|notification| {
            NotificationCard::new(
                notification.kind,
                &notification.title,
                &notification.message,
            )
            .on_dismiss(Message::DismissNotification(notification.id))
            .on_copy(Message::CopyNotification(notification.id))
            .into()
        }))
        .spacing(8)
        .width(Fill);
        container(
            container(
                scrollable(cards)
                    .anchor_bottom()
                    .height(iced::Length::Shrink),
            )
            .width(Fill)
            .max_width(420)
            .max_height(size.height * 0.6),
        )
        .width(Fill)
        .height(Fill)
        .align_right(Fill)
        .align_bottom(Fill)
        .padding(16)
        .into()
    })
    .width(Fill)
    .height(Fill)
    .into()
}
