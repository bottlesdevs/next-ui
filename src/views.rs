//! Shared screen compositions over session snapshots.

use bottles_core::ProviderState;
use iced::{
    Element, Fill, Length,
    widget::{Grid, column, container, scrollable},
};

use crate::{
    Icon,
    context::Context,
    jobs::Kind,
    theme,
    widget::{ArtworkCard, Button, CardAction, InfoCard, InfoCardKind, spacing},
};

pub fn library_grid<'a, Message: Clone + 'a>(
    ctx: &'a Context,
    query: &str,
    on_launch: impl Fn(String, String) -> Message,
) -> Element<'a, Message> {
    let query = query.to_lowercase();
    let mut cards = Vec::new();
    let mut notices = column![].spacing(spacing::SM);
    let mut providers = ctx.store.library.iter().collect::<Vec<_>>();
    providers.sort_by_key(|(provider, _)| *provider);
    for (provider, state) in providers {
        match state.as_ref() {
            ProviderState::Loaded(entries) => {
                let mut entries = entries
                    .iter()
                    .filter(|entry| entry.title.to_lowercase().contains(&query))
                    .collect::<Vec<_>>();
                entries.sort_by(|left, right| {
                    left.title
                        .cmp(&right.title)
                        .then_with(|| left.id.cmp(&right.id))
                });
                for entry in entries {
                    let launching = ctx
                        .jobs
                        .running(&Kind::Launch(provider.clone(), entry.id.clone()));
                    cards.push(
                        ArtworkCard::new(
                            &entry.title,
                            if launching { "Launching…" } else { provider },
                        )
                        .primary(
                            CardAction::new("Play", Icon::Play)
                                .loading(launching)
                                .on_press_maybe(
                                    (!launching)
                                        .then(|| on_launch(provider.clone(), entry.id.clone())),
                                ),
                        )
                        .into(),
                    );
                }
            }
            ProviderState::Loading => {
                notices = notices.push(InfoCard::new(
                    InfoCardKind::Hint,
                    "Loading library",
                    format!("Loading entries from {provider}."),
                ));
            }
            ProviderState::Failed(error) => {
                notices = notices.push(InfoCard::new(
                    InfoCardKind::Error,
                    "Library unavailable",
                    format!("{provider}: {error}"),
                ));
            }
        }
    }

    if cards.is_empty() {
        notices
            .push(InfoCard::new(
                InfoCardKind::Hint,
                "Nothing here yet",
                if query.is_empty() {
                    "Registered programs and linked storefront games will show up here."
                } else {
                    "No library titles match your search."
                },
            ))
            .into()
    } else {
        notices
            .push(
                Grid::with_children(cards)
                    .fluid(400.0)
                    .spacing(spacing::SM)
                    .height(Length::Shrink),
            )
            .into()
    }
}

pub fn page<'a, Message: Clone + 'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    max_width: f32,
) -> Element<'a, Message> {
    column![
        header.into(),
        container(scroll_content(body, max_width))
            .width(Fill)
            .height(Fill)
            .style(theme::panel)
            .clip(true),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

pub fn scroll_content<'a, Message: 'a>(
    body: impl Into<Element<'a, Message>>,
    max_width: f32,
) -> Element<'a, Message> {
    let body = container(body).width(Fill).max_width(max_width);
    scrollable(container(body).padding(24).center_x(Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .margin(12),
        ))
        .style(theme::scrollbar)
        .width(Fill)
        .height(Fill)
        .into()
}

pub fn header_button<Message>(label: &str, icon: Icon, message: Message) -> Button<'_, Message> {
    crate::widget::header_button(label, icon).on_press(message)
}
