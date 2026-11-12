//! Classic bottle compositions, read from the model.

use bottles_core::{Runner, WineBridge};
use iced::{
    Center, Element, Fill, Length, Theme,
    widget::{Grid, column, container, responsive, row, text},
};
use next_ui::{
    Icon,
    widget::{
        ActionRow, ActionTile, ArtworkCard, CardAction, CycleRow, ExpanderRow, InfoRow, LogPanel,
        PickerRow, RowGroup, SelectorRow, StatusBar, SwitcherRow, TextExt as _, TextRow, spacing,
    },
};
use uuid::Uuid;

use super::{Draft, Message};
use crate::model::{Action, Activity, Bottle, Change, Model, Verb};

/// A row for every bottle and for every bottle being created.
pub(super) fn rows(model: &Model, selected: Option<Uuid>) -> Element<'_, Message> {
    let bottles = model.bottles();
    let creating = || {
        model
            .operations
            .iter()
            .filter_map(|pending| match &pending.verb {
                Verb::Create(name) => Some((name, pending.progress.as_ref())),
                _ => None,
            })
    };
    if bottles.is_empty() && creating().next().is_none() {
        return text("No bottles yet. Add a bottle to get started.").into();
    }
    responsive(move |size| {
        let bottles = bottles.iter().map(|bottle| {
            ActionRow::new(bottle.name, Message::Select(bottle.id))
                .description(bottle.state.runner.name())
                .icon(Icon::Bottles)
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
    })
    .into()
}

pub(super) fn creation<'a>(
    draft: &'a Draft,
    model: &'a Model,
    can_create: bool,
) -> Element<'a, Message> {
    let addons = &model.sources.addons;
    let labels: Vec<_> = addons.releases::<Runner>().collect();
    let runners = labels.iter().map(|runner| runner.id()).collect::<Vec<_>>();
    let runner = SelectorRow::new("Runner", &runners, draft.runner.as_ref())
        .icon(Icon::Run)
        .placeholder(if runners.is_empty() {
            "No runners installed"
        } else {
            "Choose a runner"
        })
        .label(move |id| {
            labels.iter().find(|runner| runner.id() == *id).map_or_else(
                || id.to_string(),
                |runner| format!("{} {}", runner.name(), runner.version()),
            )
        })
        .on_selected(Message::Runner);
    let mut name = TextRow::new("Bottle Name", &draft.name).on_input(Message::Name);
    if can_create {
        name = name.on_submit(Message::Create);
    }
    let mut content = column![
        name,
        runner,
        SelectorRow::new(
            "Purpose",
            &["Gaming", "Software", "Gaming (ULWGL)", "Custom"],
            None,
        )
        .placeholder("Not available yet"),
        SelectorRow::new("Architecture", &["Win64", "Win32"], None)
            .icon(Icon::Chip)
            .placeholder("Not available yet"),
        PickerRow::new("Use Recipe").description("Choose the location"),
    ]
    .spacing(spacing::SM);
    if addons.releases::<WineBridge>().next().is_none() {
        content = content.push(InfoRow::new("WineBridge is required to create a bottle"));
    }
    content.into()
}

pub(super) fn programs<'a>(bottle: Bottle<'a>, model: &'a Model) -> Element<'a, Message> {
    let cards = bottle.programs().map(move |program| {
        let activity = model.activity(&program.target().subject());
        let launching = activity == Activity::Launching;
        ArtworkCard::new(
            program.name,
            if launching {
                "Launching…"
            } else {
                "Installed program"
            },
        )
        .primary(
            CardAction::new("Play", Icon::Play)
                .loading(launching)
                .on_press_maybe(
                    (activity == Activity::Idle)
                        .then(|| Message::Act(Action::Launch(program.target()))),
                ),
        )
        .into()
    });
    Grid::with_children(std::iter::once(new_program().into()).chain(cards))
        .fluid(400.0)
        .spacing(spacing::SM)
        .height(iced::widget::grid::aspect_ratio(3.0, 2.0))
        .into()
}

fn new_program<'a>() -> ActionTile<'a, Message> {
    let icon = container(Icon::Plus.view().width(16).height(16))
        .width(44)
        .height(44)
        .align_x(Center)
        .align_y(Center)
        .style(|theme: &Theme| {
            container::Style::default()
                .background(next_ui::theme::colors(theme).surface)
                .border(iced::Border::default().rounded(22))
        });
    let labels = column![
        text("New Program").size(17).medium(),
        text("Install or add a program.").size(14)
    ]
    .spacing(spacing::XS);
    ActionTile::new(
        container(row![icon, labels].spacing(16).align_y(Center))
            .center_x(Fill)
            .center_y(Fill),
        Message::Act(Action::AddProgram),
    )
    .width(Fill)
    .height(Fill)
    .padding(spacing::LG)
}

/// The settings of `bottle`, editable while nothing else is happening to it.
pub(super) fn settings<'a>(
    bottle: Bottle<'a>,
    rename: Option<&'a str>,
    idle: bool,
) -> Element<'a, Message> {
    const SETTINGS_TRACK_MIN_WIDTH: f32 = 300.0;
    let editing = !idle;
    let (id, name, state) = (bottle.id, bottle.name, bottle.state);
    responsive(move |size| {
        let columns = usize::from(size.width >= SETTINGS_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
        let value = rename.unwrap_or(name);
        let mut name = TextRow::new("Bottle Name", value);
        if !editing {
            name = name.on_input(Message::Rename);
            if !value.trim().is_empty() {
                name = name.on_submit(Message::SaveName);
            }
        }
        let unavailable =
            || InfoRow::new("Not available yet").description("This setting is not supported yet.");
        let bottle_group = RowGroup::new()
            .title("Bottle")
            .columns(columns)
            .row(name)
            .expander(
                ExpanderRow::with_header(
                    InfoRow::new("Runner")
                        .description(format!(
                            "{} {}",
                            state.runner.name(),
                            state.runner.version()
                        ))
                        .icon(Icon::Run),
                )
                .row(unavailable()),
            )
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
                    .on_toggle_maybe((!editing).then_some(move |enabled| {
                        Message::Act(Action::Edit(id, Change::Gamescope(enabled)))
                    })),
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
                    .on_toggle_maybe((!editing).then_some(move |enabled| {
                        Message::Act(Action::Edit(id, Change::MangoHud(enabled)))
                    })),
            );
        column![bottle_group, graphics].spacing(spacing::SM).into()
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn status<'a>(bottle: &Bottle<'a>) -> LogPanel<'a> {
    let runner = &bottle.state.runner;
    LogPanel::new(
        StatusBar::new("Ready", Icon::Lightning)
            .detail(Icon::Chip, "Win64")
            .detail(Icon::Run, format!("{} {}", runner.name(), runner.version())),
    )
}
