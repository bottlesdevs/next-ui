//! Compositions shared by the screens.

use iced::{
    Element, Fill,
    widget::{Grid, column, container, row, scrollable, stack, text},
};
use next_ui::{
    Icon, theme,
    widget::{
        ArtworkCard, Button, ButtonKind, CardAction, InfoCard, InfoCardKind, Popover, PopoverItem,
        ProgressBar, ProgressRing, TextExt as _, spacing,
    },
};

use next_ui::{layout::PaneContext, widget::HeaderBar};

use crate::{
    manage_profiles,
    model::{Action, Activity, Experience, Listing, Model, Target},
};

/// A card for every program and library entry whose title contains `query`.
///
/// A card has a details button when `on_details` returns a message for it.
pub fn library_grid<'a, Message: Clone + 'a>(
    model: &'a Model,
    query: &str,
    on_launch: impl Fn(Target) -> Message,
    on_details: impl Fn(&Target) -> Option<Message>,
) -> Element<'a, Message> {
    let query = query.to_lowercase();
    let mut notices = column![].spacing(spacing::SM);
    for (plugin, listing) in &model.sources.listings {
        match listing {
            Listing::Loaded(_) => {}
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

    let cards = model
        .entries()
        .into_iter()
        .filter(|entry| entry.title.to_lowercase().contains(&query))
        .map(|entry| {
            let activity = model.activity(&entry.target.subject());
            let launching = activity == Activity::Launching;
            let details = on_details(&entry.target);
            let card = ArtworkCard::new(
                entry.title,
                match activity {
                    Activity::Launching => "Launching…",
                    Activity::Running => "Running",
                    _ => entry.source,
                },
            )
            .primary(
                CardAction::new("Play", Icon::Play)
                    .loading(launching)
                    .on_press_maybe((activity == Activity::Idle).then(|| on_launch(entry.target))),
            );
            match details {
                Some(message) => card
                    .menu(CardAction::new("Details", Icon::EllipsisVertical).on_press(message))
                    .into(),
                None => card.into(),
            }
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

/// A page whose body fills the panel without scrolling, e.g. a
/// [`StatusPage`](next_ui::widget::StatusPage) that centres itself.
pub fn status_page<'a, Message: Clone + 'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        header.into(),
        container(body)
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
    next_ui::widget::header_button(label, icon).on_press(message)
}

/// The app menu at the start of the main header: Preferences, and switching
/// to the other experience when this build has it.
pub fn app_menu<'a, Message: Clone + 'a>(
    current: Experience,
    preferences: Message,
    switch: impl Fn(Experience) -> Message,
) -> Element<'a, Message> {
    let other = match current {
        Experience::Next => Experience::Classic,
        Experience::Classic => Experience::Next,
    };
    let mut menu = Popover::new(next_ui::widget::header_button(
        "App menu",
        Icon::EllipsisVertical,
    ))
    .item(
        PopoverItem::new("Preferences")
            .icon(Icon::Gear)
            .on_select(preferences),
    );
    if other.is_available() {
        menu = menu.item(
            PopoverItem::new(match other {
                Experience::Next => "Switch to Next",
                Experience::Classic => "Switch to Classic",
            })
            .icon(Icon::Refresh)
            .on_select(switch(other)),
        );
    }
    menu.into()
}

/// Adds the app-wide controls to the end of `header` when `pane` reaches the
/// window's end edge: Downloads while releases download, then the profile
/// button.
pub fn app_controls<'a, Message: Clone + 'a>(
    header: HeaderBar<'a, Message>,
    model: &'a Model,
    pane: PaneContext,
    profiles: impl Fn(manage_profiles::Message) -> Message + 'a,
    act: impl Fn(Action) -> Message,
) -> HeaderBar<'a, Message> {
    if !pane.is_at_end() {
        return header;
    }
    let header = match downloads(model, act) {
        Some(downloads) => header.end(downloads),
        None => header,
    };
    header.end(manage_profiles::button(model).map(profiles))
}

/// The Downloads button and its popover, while any release downloads.
fn downloads<'a, Message: Clone + 'a>(
    model: &'a Model,
    act: impl Fn(Action) -> Message,
) -> Option<Element<'a, Message>> {
    let downloads = model.downloads();
    if downloads.is_empty() {
        return None;
    }
    let overall = downloads
        .iter()
        .map(|download| download.progress.unwrap_or(0.0))
        .sum::<f32>()
        / downloads.len() as f32;
    let trigger = Button::custom(stack![
        ProgressRing::new(overall),
        container(Icon::Download.view().width(12).height(12)).center(24),
    ])
    .kind(ButtonKind::Transparent);
    let rows = downloads
        .into_iter()
        .map(|download| -> Element<'a, Message> {
            let mut body = column![
                text(download.name).size(17).medium(),
                text(download.status).size(14),
            ]
            .spacing(4)
            .width(Fill);
            if let Some(progress) = download.progress {
                body = body.push(ProgressBar::new(progress).percentage());
            }
            container(
                row![
                    body,
                    next_ui::widget::header_button("Cancel download", Icon::Cross)
                        .on_press(act(Action::CancelDownload(download.id))),
                ]
                .spacing(spacing::SM)
                .align_y(iced::Alignment::Center),
            )
            .padding([spacing::SM, spacing::SM])
            .into()
        });
    Some(
        Popover::new(trigger)
            .content(column(rows).spacing(spacing::XS))
            .width(420.0)
            .into(),
    )
}
