//! Classic bottle compositions derived from the session snapshots and jobs.

use bottles_core::BottleState;
use iced::{
    Center, Element, Fill, Length, Theme,
    widget::{Grid, column, container, responsive, row, text},
};
use next_ui::{
    Icon,
    widget::{
        ActionRow, ActionTile, ArtworkCard, CardAction, CycleRow, ExpanderRow, InfoRow, RowGroup,
        StatusBar, SwitcherRow, TextExt as _, TextRow, spacing,
    },
};
use uuid::Uuid;

use super::{Draft, Message};
use crate::{
    jobs::{Jobs, Kind},
    store::Store,
};

pub(super) fn rows<'a>(
    store: &'a Store,
    jobs: &'a Jobs,
    selected: Option<Uuid>,
) -> Element<'a, Message> {
    responsive(move |size| {
        let bottles = store.bottles.iter().map(|bottle| {
            ActionRow::new(bottle.name(), Message::Select(bottle.id()))
                .description(bottle.config().runner.name())
                .icon(Icon::Bottles)
                .selected(selected == Some(bottle.id()))
                .into()
        });
        let creating = jobs.map.values().filter_map(|job| {
            let Kind::CreateBottle(name) = &job.kind else {
                return None;
            };
            Some(
                ActionRow::progress(
                    name,
                    job.progress
                        .as_ref()
                        .and_then(bottles_core::Progress::fraction)
                        .unwrap_or(0.0),
                )
                .description(
                    job.progress
                        .as_ref()
                        .map_or_else(|| "Preparing".into(), |progress| progress.stage.to_string()),
                )
                .icon(Icon::Bottles)
                .into(),
            )
        });
        if store.bottles.is_empty()
            && !jobs
                .map
                .values()
                .any(|job| matches!(job.kind, Kind::CreateBottle(_)))
        {
            return text("No bottles yet. Add a bottle to get started.").into();
        }
        container(
            container(
                Grid::with_children(bottles.chain(creating))
                    .columns(usize::from(size.width >= 600.0 + spacing::SM) + 1)
                    .spacing(spacing::SM)
                    .height(Length::Shrink),
            )
            .width(Fill)
            .max_width(720),
        )
        .center_x(Fill)
        .into()
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn creation<'a>(
    draft: &'a Draft,
    store: &'a Store,
    can_create: bool,
) -> Element<'a, Message> {
    let mut runners = RowGroup::new().title("Runner");
    for runner in store.addons.runners() {
        runners = runners.row(
            ActionRow::new(
                format!("{} {}", runner.name(), runner.version()),
                Message::Runner(runner.id()),
            )
            .icon(Icon::Run)
            .selected(draft.runner == Some(runner.id())),
        );
    }
    if store.addons.runners().is_empty() {
        runners = runners.row(InfoRow::new("No runners installed"));
    }
    let mut name = TextRow::new("Bottle Name", &draft.name)
        .icon(Icon::Person)
        .on_input(Message::Name);
    if can_create {
        name = name.on_submit(Message::Create);
    }
    let mut content = column![name, runners].spacing(spacing::MD);
    if store.addons.winebridges().is_empty() {
        content = content.push(InfoRow::new("WineBridge is required to create a bottle"));
    }
    content.into()
}

pub(super) fn programs<'a>(bottle: &'a BottleState, jobs: &'a Jobs) -> Element<'a, Message> {
    let cards = bottle.programs().map(|(id, program)| {
        let launching = jobs.running(&Kind::Launch(
            "bottles".into(),
            format!("{}/{id}", bottle.id()),
        ));
        ArtworkCard::new(
            program.name(),
            if launching {
                "Launching…"
            } else {
                "Installed program"
            },
        )
        .primary(
            CardAction::new("Play", Icon::Play)
                .loading(launching)
                .on_press_maybe((!launching && !busy(jobs, bottle.id())).then_some(
                    Message::LaunchProgram {
                        bottle: bottle.id(),
                        program: id,
                    },
                )),
        )
        .into()
    });
    Grid::with_children(std::iter::once(new_program().into()).chain(cards))
        .fluid(400.0)
        .spacing(spacing::MD)
        .height(Length::Shrink)
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
        container(row![icon, labels].spacing(16).align_y(Center)).center_x(Fill),
        Message::NewProgram,
    )
    .width(Fill)
    .padding([72.0, spacing::LG])
}

pub(super) fn settings<'a>(
    bottle: &'a BottleState,
    rename: Option<&'a str>,
    jobs: &Jobs,
) -> Element<'a, Message> {
    const SETTINGS_TRACK_MIN_WIDTH: f32 = 300.0;
    let editing = busy(jobs, bottle.id());
    responsive(move |size| {
        let columns = usize::from(size.width >= SETTINGS_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
        let value = rename.unwrap_or_else(|| bottle.name());
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
                            bottle.config().runner.name(),
                            bottle.config().runner.version()
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
        let id = bottle.id();
        let wrappers = &bottle.config().wrappers;
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
                    .on_toggle_maybe(
                        (!editing).then_some(move |enabled| Message::ToggleGamescope(id, enabled)),
                    ),
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
                    .on_toggle_maybe(
                        (!editing).then_some(move |enabled| Message::ToggleMangoHud(id, enabled)),
                    ),
            );
        column![bottle_group, graphics].spacing(spacing::SM).into()
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn busy(jobs: &Jobs, bottle: Uuid) -> bool {
    [Kind::Edit(bottle), Kind::Stop(bottle), Kind::Delete(bottle)]
        .iter()
        .any(|kind| jobs.running(kind))
}

pub(super) fn status(bottle: &BottleState) -> StatusBar<'_> {
    StatusBar::new("Ready", Icon::Bottles).detail(Icon::Run, bottle.config().runner.name())
}
