//! Classic bottle compositions, read from the model.

use bottles_core::{Slot, WineArch};
use iced::{
    Fill, Length,
    widget::{Grid, column, container, responsive, text},
};
use next_ui::{
    Element, Icon,
    layout::PageBody,
    widget::{
        ActionRow, ActionTile, ArtworkCard, Button, ButtonKind, CardAction, CycleRow, ExpanderRow,
        InfoRow, LogPanel, PickerRow, ProgressBar, RowGroup, SelectorItem, SelectorRow, StatusBar,
        SwitcherRow, TextRow, spacing, text_size,
    },
};
use uuid::Uuid;

use super::{Draft, Message};
use crate::model::{Action, Activity, AvailableRelease, Bottle, Change, Model, Then, Verb};

/// A row for every bottle and for every bottle being created.
pub(super) fn rows(model: &Model, selected: Option<Uuid>) -> PageBody<'_, Message> {
    let bottles = model.bottles();
    let creating = || {
        model
            .operations
            .iter()
            .filter_map(|(_, pending)| match (&pending.verb, &pending.then) {
                (Verb::Create, Some(Then::RecordBottle { name })) => {
                    Some((name, pending.progress.as_ref()))
                }
                _ => None,
            })
    };
    if bottles.is_empty() && creating().next().is_none() {
        let empty = next_ui::widget::StatusPage::new("No bottles yet")
            .description(
                "A bottle is a Windows environment for the programs you install. Create one to \
                 get started.",
            )
            .child(
                next_ui::widget::Button::new("New Bottle")
                    .icon(Icon::Plus)
                    .kind(next_ui::widget::ButtonKind::Accent)
                    .on_press(Message::New),
            );
        return PageBody::placeholder(empty);
    }
    PageBody::content(responsive(move |size| {
        let bottles = bottles.iter().map(|bottle| {
            ActionRow::new(bottle.name, Message::Select(bottle.id))
                .description(bottle.state.runner.name())
                .icon(Icon::Bottles)
                .trailing(Icon::ArrowRight.view())
                .selected(selected == Some(bottle.id))
                .into()
        });
        let creating = creating().map(|(name, progress)| {
            ActionRow::progress(
                name,
                progress
                    .and_then(bottles_core::Progress::fraction)
                    .unwrap_or(0.0),
            )
            .description(
                progress.map_or_else(|| "Preparing".into(), |progress| progress.stage.to_string()),
            )
            .icon(Icon::Bottles)
            .into()
        });
        let rows = bottles
            .chain(creating)
            .collect::<Vec<Element<'_, Message>>>();
        let max_width = 600.0 + spacing::SM;
        let columns = ((size.width + spacing::SM) / (max_width + spacing::SM)).ceil();
        let occupied_columns = (rows.len() as f32).min(columns);
        // Remove empty grid columns without changing the fluid cell widths.
        let width = (size.width + spacing::SM) / columns * occupied_columns - spacing::SM;
        container(
            Grid::with_children(rows)
                .fluid(max_width)
                .width(width)
                .spacing(spacing::SM)
                .height(Length::Shrink),
        )
        .center_x(Fill)
        .into()
    }))
}

pub(super) fn creation<'a>(
    draft: &'a Draft,
    model: &'a Model,
    can_create: bool,
) -> Element<'a, Message> {
    let runners = model.runners();
    let runner = SelectorRow::new("Runner")
        .icon(Icon::Run)
        .placeholder(if runners.is_empty() {
            "No runners installed"
        } else {
            "Choose a runner"
        })
        .group(
            None,
            runners.iter().map(|runner| {
                SelectorItem::choice(format!("{} {}", runner.name(), runner.version()))
                    .selected(draft.runner == Some(runner.id()))
                    .on_press(Message::Runner(runner.id()))
            }),
        );
    let mut name = TextRow::new("Bottle Name", &draft.name).on_input(Message::Name);
    if can_create {
        name = name.on_submit(Message::Create);
    }
    let mut content = column![
        name,
        runner,
        SelectorRow::new("Purpose")
            .placeholder("Not available yet")
            .group(
                None,
                ["Gaming", "Software", "Gaming (ULWGL)", "Custom"].map(SelectorItem::choice),
            ),
        SelectorRow::new("Architecture").icon(Icon::Chip).group(
            None,
            [WineArch::Win64, WineArch::Win32].map(|arch| {
                SelectorItem::choice(arch_label(arch))
                    .selected(draft.arch == arch)
                    .on_press(Message::Arch(arch))
            }),
        ),
        PickerRow::new("Use Recipe").description("Choose the location"),
    ]
    .spacing(spacing::SM);
    if model.winebridge().is_none() {
        content = content.push(InfoRow::new("WineBridge is required to create a bottle"));
    }
    content.into()
}

pub(super) fn programs<'a>(bottle: Bottle<'a>, model: &'a Model) -> Element<'a, Message> {
    let cards = bottle.programs().map(move |program| {
        let activity = model.activity(&program.target().subject());
        ArtworkCard::new(
            program.name,
            match activity {
                Activity::Launching => "Launching…",
                Activity::Running => "Running",
                Activity::Stopping => "Stopping…",
                Activity::Idle | Activity::Busy => "Installed program",
            },
        )
        .primary(if activity == Activity::Running {
            CardAction::new("Force stop", Icon::Stop).on_press(Message::Act(Action::Kill {
                prefix: program.prefix,
                group: program.group,
            }))
        } else {
            CardAction::new("Play", Icon::Play)
                .loading(activity == Activity::Launching)
                .on_press_maybe(
                    (activity == Activity::Idle)
                        .then(|| Message::Act(Action::Launch(program.target()))),
                )
        })
        .into()
    });
    Grid::with_children(std::iter::once(new_program(bottle.id).into()).chain(cards))
        .fluid(400.0)
        .spacing(spacing::SM)
        .height(iced::widget::grid::aspect_ratio(3.0, 2.0))
        .into()
}

fn new_program<'a>(bottle: Uuid) -> ActionTile<'a, Message> {
    ActionTile::new(Icon::Plus, "New Program", "Install or add a program.")
        .on_press(Message::AddProgram(Some(bottle)))
}

/// The settings of `bottle`.
pub(super) fn settings<'a>(
    bottle: Bottle<'a>,
    model: &'a Model,
    rename: Option<&'a str>,
) -> Element<'a, Message> {
    const SETTINGS_TRACK_MIN_WIDTH: f32 = 300.0;
    let (id, name, state) = (bottle.id, bottle.name, bottle.state);
    responsive(move |size| {
        let columns = usize::from(size.width >= SETTINGS_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
        let value = rename.unwrap_or(name);
        let mut name = TextRow::new("Bottle Name", value).on_input(Message::Rename);
        if !value.trim().is_empty() {
            name = name.on_submit(Message::SaveName);
        }
        let unavailable =
            || InfoRow::new("Not available yet").description("This setting is not supported yet.");
        let bottle_group = RowGroup::new()
            .title("Bottle")
            .columns(columns)
            .row(name)
            .row(runner(model, bottle))
            .expander(
                ExpanderRow::new("Dependencies")
                    .description("Install fonts, codecs, libraries...")
                    .row(unavailable()),
            )
            .expander(
                ExpanderRow::new("Drives")
                    .description("Define your custom drives")
                    .row(unavailable()),
            );
        let wrappers = &state.wrappers;
        let graphics = RowGroup::new()
            .title("Graphics")
            .columns(columns)
            .row(component(model, bottle, Slot::Dxvk, "DXVK"))
            .row(component(model, bottle, Slot::Vkd3d, "VKD3D"))
            .row(component(model, bottle, Slot::LatencyFlex, "LatencyFlex"))
            .row(SwitcherRow::new("DLSS", false).description("Deep Learning Super Sampling"))
            .row(SwitcherRow::new("vkBasalt", false).description("Add post-processing effects"))
            .row(
                SwitcherRow::new("Discrete GPU", false).description("Force use your dedicated GPU"),
            )
            .expander(
                ExpanderRow::with_header(
                    SwitcherRow::new("FSR", false).description("FidelityFX Super Resolution"),
                )
                .columns(2)
                .row(ActionRow::new("Quality", None).description("Balanced"))
                .row(CycleRow::new("Sharpening", "5")),
            )
            .row(
                SwitcherRow::new("Gamescope", wrappers.gamescope.enabled)
                    .description("Use the SteamOS compositor")
                    .on_toggle(move |enabled| {
                        Message::Act(Action::Edit(id, Change::Gamescope(enabled)))
                    }),
            )
            .expander(
                ExpanderRow::with_header(
                    SwitcherRow::new("Display Settings", false)
                        .description("Resolution and other options"),
                )
                .row(unavailable()),
            )
            .row(
                SwitcherRow::new("MangoHud", wrappers.mangohud.enabled)
                    .description("Show a performance overlay")
                    .on_toggle(move |enabled| {
                        Message::Act(Action::Edit(id, Change::MangoHud(enabled)))
                    }),
            );
        column![bottle_group, graphics].spacing(spacing::SM).into()
    })
    .height(Length::Shrink)
    .into()
}

/// A row for `slot`: its switch turns the component on or off, and its
/// options pick the downloaded release, or download one of the newest the
/// catalogs offer, which is selected once downloaded.
fn component<'a>(
    model: &'a Model,
    bottle: Bottle<'a>,
    slot: Slot,
    title: &'a str,
) -> Element<'a, Message> {
    let prefix = bottle.id;
    let downloaded = model.component_releases(slot);
    let current = bottle
        .state
        .component(slot)
        .map(|installed| installed.addon.id());
    let available = model.available_components(slot);
    // Turning it on uses the newest downloaded release, or downloads the
    // newest one; with neither, the switch stays off.
    let turn_on = downloaded
        .first()
        .map(|(release, _)| Action::UseRelease {
            prefix,
            release: *release,
        })
        .or_else(|| {
            available
                .iter()
                .find(|release| release.download.is_none())
                .map(|release| Action::FetchRelease {
                    prefix,
                    release: release.id,
                })
        });
    let toggle = if current.is_some() {
        Some(Action::RemoveComponent { prefix, slot })
    } else {
        turn_on
    };
    SelectorRow::new(title)
        .placeholder("Off")
        .popup()
        .group("Downloaded", downloaded_items(downloaded, current, prefix))
        .group("Available", available_items(available, prefix))
        .switch(
            current.is_some(),
            toggle.map(|action| move |_| Message::Act(action.clone())),
        )
        .into()
}

/// A picker for the bottle's runner: the downloaded runners it can use and
/// the newest ones the catalogs offer, which are selected once downloaded.
fn runner<'a>(model: &'a Model, bottle: Bottle<'a>) -> Element<'a, Message> {
    let prefix = bottle.id;
    let current = &bottle.state.runner;
    let mut downloaded = model.runner_releases(bottle.state.arch);
    // The runner in use stays listed even if it was removed since.
    if !downloaded.iter().any(|(id, _)| *id == current.id()) {
        downloaded.insert(
            0,
            (
                current.id(),
                format!("{} {}", current.name(), current.version()),
            ),
        );
    }
    SelectorRow::new("Runner")
        .icon(Icon::Run)
        .popup()
        .group(
            "Downloaded",
            downloaded_items(downloaded, Some(current.id()), prefix),
        )
        .group(
            "Available",
            available_items(model.available_runners(), prefix),
        )
        .into()
}

/// Downloaded releases as options: picking one uses it in bottle `prefix`.
fn downloaded_items<'a>(
    releases: Vec<(Uuid, String)>,
    current: Option<Uuid>,
    prefix: Uuid,
) -> impl Iterator<Item = SelectorItem<'a, Message>> {
    releases.into_iter().map(move |(release, label)| {
        SelectorItem::choice(label)
            .selected(current == Some(release))
            .on_press(Message::Act(Action::UseRelease { prefix, release }))
    })
}

/// Releases the catalogs offer: picking one downloads it, then uses it in
/// bottle `prefix`. One downloading shows its progress, and picking it again
/// stops the download.
fn available_items<'a>(
    releases: Vec<AvailableRelease>,
    prefix: Uuid,
) -> impl Iterator<Item = SelectorItem<'a, Message>> {
    releases.into_iter().map(move |release| {
        let item = SelectorItem::action(release.label);
        match release.download {
            Some(download) => {
                let fraction = download.fraction().unwrap_or(0.0);
                item.detail(ProgressBar::new(fraction))
                    .trailing(text(format!("{:.0}%", fraction * 100.0)).size(text_size::SMALL))
                    .on_press(Message::Act(Action::CancelDownload(release.id)))
            }
            None => {
                let fetch = Message::Act(Action::FetchRelease {
                    prefix,
                    release: release.id,
                });
                // The button shows what picking the row does; the row takes
                // the press.
                item.trailing(
                    Button::custom(text("Download").size(text_size::SMALL))
                        .trailing_icon(Icon::ArrowRight)
                        .icon_size(14.0)
                        .kind(ButtonKind::Secondary)
                        .compact()
                        .on_press(fetch.clone()),
                )
                .on_press(fetch)
            }
        }
    })
}

fn arch_label(arch: WineArch) -> &'static str {
    match arch {
        WineArch::Win64 => "Win64",
        WineArch::Win32 => "Win32",
    }
}

pub(super) fn status<'a>(bottle: &Bottle<'a>, activity: Activity) -> LogPanel<'a> {
    let runner = &bottle.state.runner;
    let (label, icon) = match activity {
        Activity::Running => ("Running", Icon::Lightning),
        Activity::Idle => ("Stopped", Icon::Power),
        Activity::Stopping => ("Stopping", Icon::Power),
        Activity::Launching | Activity::Busy => ("Busy", Icon::Lightning),
    };
    LogPanel::new(
        StatusBar::new(label, icon)
            .detail(Icon::Chip, arch_label(bottle.state.arch))
            .detail(Icon::Run, format!("{} {}", runner.name(), runner.version())),
    )
}
