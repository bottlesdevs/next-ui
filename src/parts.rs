//! Compositions shared by the screens.

use iced::{
    Element, Fill,
    widget::{Grid, Space, column, container, row, stack, text},
};
use next_ui::{
    Icon, theme,
    widget::{
        ArtworkCard, Button, ButtonKind, CardAction, InfoCard, InfoCardKind, ListRow, Popover,
        ProgressBar, ProgressRing, StatusPage, TextExt as _, spacing,
    },
};

use next_ui::{
    layout::{PageBody, PaneContext},
    widget::HeaderBar,
};

use crate::{
    app::Control,
    manage_profiles,
    model::{Activity, Download, Experience, Listing, Model, Target},
};

/// A card for every program and library entry whose title contains `query`,
/// or a status page when there are none.
///
/// A card has a details button when `on_details` returns a message for it.
/// `on_add` adds a program from the computer.
pub fn library_grid<'a, Message: Clone + 'a>(
    model: &'a Model,
    query: &str,
    on_launch: impl Fn(Target) -> Message,
    on_details: impl Fn(&Target) -> Option<Message>,
    on_add: Message,
    on_install: impl Fn(&'static str) -> Message,
    searchable: bool,
) -> PageBody<'a, Message> {
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
        let empty: Element<'a, Message> = if query.is_empty() {
            suggestions(model, on_install, on_add, searchable)
        } else {
            StatusPage::new("No results")
                .description("Nothing in your library matches your search.")
                .into()
        };
        PageBody::placeholder(column![notices, empty].height(Fill))
    } else {
        PageBody::content(
            notices.push(
                Grid::with_children(cards)
                    .fluid(400.0)
                    .spacing(spacing::SM)
                    .height(iced::widget::grid::aspect_ratio(3.0, 2.0)),
            ),
        )
    }
}

/// An empty library: programs to install, then adding one from the computer.
fn suggestions<'a, Message: Clone + 'a>(
    model: &Model,
    on_install: impl Fn(&'static str) -> Message,
    on_add: Message,
    searchable: bool,
) -> Element<'a, Message> {
    let cards = model.suggestions().iter().map(|&(name, description)| {
        let art = container(Space::new())
            .width(Fill)
            .height(120)
            .style(|theme| {
                container::Style::default()
                    .background(theme::colors(theme).selected)
                    .border(
                        iced::Border::default()
                            .rounded(iced::border::top(next_ui::widget::radius::ROW)),
                    )
            });
        let footer = row![
            column![
                text(name).size(18).medium(),
                text(description).size(15).muted()
            ]
            .spacing(4)
            .width(Fill),
            Button::new("Install")
                .trailing_icon(Icon::Arrow)
                .icon_rotation(std::f32::consts::PI)
                .icon_size(16.0)
                .kind(ButtonKind::Secondary)
                .on_press(on_install(name)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center);
        container(column![art, container(footer).padding([18, 22])])
            .width(Fill)
            .clip(true)
            .style(|theme| {
                container::Style::default()
                    .background(theme::colors(theme).card)
                    .border(iced::Border::default().rounded(next_ui::widget::radius::ROW))
            })
            .into()
    });
    let add = Button::new("Add a program from your computer")
        .icon(Icon::Folder)
        .kind(ButtonKind::Transparent)
        .on_press(on_add);
    let content = column![
        text("Your library is empty").size(26).medium(),
        text(if searchable {
            "Install something to get started, or search for anything above."
        } else {
            "Install something to get started, or add a program you already have."
        })
        .size(17)
        .muted(),
        row(cards).spacing(16),
        add,
    ]
    .spacing(spacing::MD)
    .max_width(820);
    container(content).center(Fill).padding(24).into()
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
        ListRow::labeled("Preferences")
            .leading_icon(Icon::Gear)
            .on_press(preferences),
    );
    if other.is_available() {
        menu = menu.item(
            ListRow::labeled(match other {
                Experience::Next => "Switch to Next",
                Experience::Classic => "Switch to Classic",
            })
            .leading_icon(Icon::Refresh)
            .on_press(switch(other)),
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
    control: impl Fn(Control) -> Message + 'a,
) -> HeaderBar<'a, Message> {
    if !pane.is_at_end() {
        return header;
    }
    let header = match downloads(model, &control) {
        Some(downloads) => header.end(downloads),
        None => header,
    };
    header
        .end(manage_profiles::button(model).map(move |message| control(Control::Profiles(message))))
}

/// The Downloads button and its popover, while any release downloads.
fn downloads<'a, Message: Clone + 'a>(
    model: &'a Model,
    control: impl Fn(Control) -> Message,
) -> Option<Element<'a, Message>> {
    let downloads = model.downloads();
    if downloads.is_empty() {
        return None;
    }
    let overall = downloads
        .iter()
        .map(|download| download.fraction().unwrap_or(0.0))
        .sum::<f32>()
        / downloads.len() as f32;
    let trigger = Button::custom(stack![
        ProgressRing::new(overall),
        container(Icon::Download.view().width(12).height(12)).center(24),
    ])
    .kind(ButtonKind::Transparent);
    // Each download is a card of its own, set a step darker into the popover:
    // its name and status over the progress bar, the percentage beside it.
    let rows = downloads
        .into_iter()
        .map(|download| -> Element<'a, Message> {
            let status = download_status(&download);
            let mut body = column![
                text(download_name(&download)).size(18).medium(),
                text(status).size(15).style(|theme| text::Style {
                    color: Some(theme::colors(theme).accent_muted),
                }),
            ]
            .spacing(6)
            .width(Fill);
            let mut card = row![].spacing(spacing::SM).align_y(iced::Alignment::Center);
            if let Some(progress) = download.fraction() {
                body = body.push(
                    container(ProgressBar::new(progress)).padding(iced::Padding::ZERO.top(6)),
                );
                card = card.push(body).push(
                    text(format!("{}%", (progress * 100.0).round()))
                        .size(17)
                        .medium(),
                );
            } else {
                card = card.push(body);
            }
            container(
                card.push(
                    next_ui::widget::header_button("Cancel download", Icon::Cross)
                        .on_press(control(Control::CancelDownload(download.id))),
                ),
            )
            .padding(iced::Padding {
                top: 16.0,
                right: 8.0,
                bottom: 18.0,
                left: 24.0,
            })
            .style(|theme| {
                container::Style::default()
                    .background(theme::colors(theme).raised)
                    .border(iced::Border::default().rounded(next_ui::widget::radius::BOX))
            })
            .into()
        });
    Some(
        Popover::new(trigger)
            .content(column(rows).spacing(8))
            .width(420.0)
            .into(),
    )
}

/// A download's release name, or a stand-in once the catalogs drop it.
fn download_name(download: &Download) -> String {
    download
        .name
        .clone()
        .unwrap_or_else(|| "Release".to_owned())
}

/// What a download is doing, e.g. "412 of 640 MB" or "Extracting".
fn download_status(download: &Download) -> String {
    download.progress.as_ref().map_or_else(
        || "Waiting".to_owned(),
        |progress| match progress.transfer {
            Some(bottles_core::Transfer {
                current,
                total: Some(total),
            }) if total > 0 => format!("{} of {} MB", current / 1_000_000, total / 1_000_000),
            _ => progress.stage.to_string(),
        },
    )
}
