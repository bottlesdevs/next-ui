//! Shared screen compositions over session snapshots.

use iced::{
    Element, Fill,
    widget::{Grid, column, container, scrollable},
};

use crate::{
    Icon,
    context::Context,
    jobs::Kind,
    library::{Entry, Listing},
    theme,
    widget::{ArtworkCard, Button, CardAction, InfoCard, InfoCardKind, spacing},
};

pub fn library_grid<'a, Message: Clone + 'a>(
    ctx: &'a Context,
    query: &str,
    on_launch: impl Fn(Entry) -> Message,
) -> Element<'a, Message> {
    let query = query.to_lowercase();
    let mut entries: Vec<(&str, &str, Entry)> = Vec::new();
    for bottle in &ctx.store.bottles {
        for (id, program) in bottle.programs() {
            entries.push((
                program.name(),
                bottle.name(),
                Entry::Bottle(bottle.id(), id),
            ));
        }
    }
    #[cfg(feature = "fvs")]
    for program in &ctx.store.programs {
        entries.push((program.name(), "Program", Entry::Program(program.id())));
    }
    let mut notices = column![].spacing(spacing::SM);
    for (plugin, listing) in &ctx.store.plugins {
        match listing {
            Listing::Loaded(listed) => {
                for entry in listed {
                    entries.push((
                        &entry.title,
                        plugin,
                        Entry::Plugin(plugin.clone(), entry.id.clone()),
                    ));
                }
            }
            Listing::Loading => {
                notices = notices.push(InfoCard::new(
                    InfoCardKind::Hint,
                    "Loading library",
                    format!("Loading entries from {plugin}."),
                ));
            }
            Listing::Failed(error) => {
                notices = notices.push(InfoCard::new(
                    InfoCardKind::Error,
                    "Library unavailable",
                    format!("{plugin}: {error}"),
                ));
            }
        }
    }
    entries.retain(|(title, _, _)| title.to_lowercase().contains(&query));
    entries.sort_by(|left, right| (left.0, left.1).cmp(&(right.0, right.1)));

    let cards = entries
        .into_iter()
        .map(|(title, source, entry)| {
            let launching = ctx.jobs.running(&Kind::Launch(entry.clone()));
            ArtworkCard::new(title, if launching { "Launching…" } else { source })
                .primary(
                    CardAction::new("Play", Icon::Play)
                        .loading(launching)
                        .on_press_maybe((!launching).then(|| on_launch(entry))),
                )
                .into()
        })
        .collect::<Vec<_>>();

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
                    .height(iced::widget::grid::aspect_ratio(3.0, 2.0)),
            )
            .into()
    }
}

pub fn page<'a, Message: Clone + 'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        header.into(),
        container(scroll_content(body))
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
) -> Element<'a, Message> {
    scrollable(container(body).padding(24).width(Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .margin(6),
        ))
        .style(theme::scrollbar)
        .width(Fill)
        .height(Fill)
        .into()
}

pub fn header_button<Message>(label: &str, icon: Icon, message: Message) -> Button<'_, Message> {
    crate::widget::header_button(label, icon).on_press(message)
}
