//! Classic presentation for the shared library workflow.

use bottles_core::{SearchEntry, SearchSource};
use iced::{
    Element, Fill, Length,
    widget::{Grid, column, container},
};

use crate::{
    Icon,
    features::library::{Message, State, Status},
    widget::{ArtworkCard, CardAction, InfoCard, InfoCardKind, Search, spacing},
};

const NARROW_CONTENT_MAX_WIDTH: f32 = 500.0;

pub fn view(state: &State) -> Element<'_, Message> {
    let search = centered_narrow(Search::new(
        "Search library",
        state.query(),
        Message::QueryChanged,
    ));

    let notice = match state.status() {
        Status::Idle => Some((
            "No active profile",
            "Sign in to a profile to see its library.",
        )),
        Status::Loading => Some((
            "Loading library",
            "Loading games from this profile's linked storefronts.",
        )),
        Status::Loaded => None,
    };
    if let Some((title, body)) = notice {
        return column![
            search,
            centered_narrow(InfoCard::new(InfoCardKind::Hint, title, body).width(Fill))
        ]
        .spacing(12)
        .into();
    }

    let mut content = column![search].spacing(12);
    if let Some(error) = state.last_error() {
        content = content.push(centered_narrow(
            InfoCard::new(InfoCardKind::Error, "Program launch failed", error).width(Fill),
        ));
    }
    if state.entries().is_empty() {
        return content
            .push(centered_narrow(
                InfoCard::new(
                    InfoCardKind::Hint,
                    "Nothing here yet",
                    "Registered programs and linked storefront games will show up here.",
                )
                .width(Fill),
            ))
            .into();
    }

    let rows = Grid::with_children(state.entries().iter().map(entry_card))
        .fluid(400.0)
        .spacing(spacing::MD)
        .height(Length::Shrink);

    content.push(rows).into()
}

fn centered_narrow<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        container(content)
            .width(Fill)
            .max_width(NARROW_CONTENT_MAX_WIDTH),
    )
    .center_x(Fill)
    .into()
}

fn entry_card(entry: &SearchEntry) -> Element<'_, Message> {
    ArtworkCard::new(entry.title(), entry.source_name())
        .menu(CardAction::new("More actions", Icon::EllipsisVertical))
        .primary(
            CardAction::new("Play", Icon::Play).on_press_maybe(match entry.source() {
                SearchSource::Installed(item) => Some(Message::Launch(item.clone())),
                _ => None,
            }),
        )
        .into()
}
