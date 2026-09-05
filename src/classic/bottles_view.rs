//! Classic presentation for bottles, creation, and installed programs.

use std::sync::Arc;

use bottles_core::{Bottle, BottleState};
use iced::{
    Center, Element, Length, Theme,
    widget::{Grid, column, container, responsive, row, text},
};
use uuid::Uuid;

use crate::{
    Icon,
    features::bottles::{Message, State},
    widget::{
        ActionRow, ActionTile, ArtworkCard, CardAction, InfoCard, InfoCardKind, LogPanel,
        PickerRow, SelectorRow, TextExt as _, TextRow, spacing,
    },
};

const BOTTLE_LIST_MAX_WIDTH: f32 = 720.0;
const BOTTLE_TRACK_MIN_WIDTH: f32 = 300.0;
const PURPOSES: [&str; 4] = ["Gaming", "Software", "Gaming (ULWGL)", "Custom"];
const ARCHITECTURES: [&str; 2] = ["Win64", "Win32"];

pub fn rows<'a, Msg: 'static + Clone>(
    bottle_states: &'a [Arc<BottleState>],
    selected_id: Option<Uuid>,
    on_select: impl Fn(Uuid) -> Msg + 'a,
) -> Element<'a, Msg> {
    responsive(move |size| {
        let columns = usize::from(size.width >= BOTTLE_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
        let rows = bottle_states.iter().map(|state| {
            ActionRow::new(state.name(), on_select(state.id()))
                .description(state.runner().name())
                .icon(Icon::Bottles)
                .selected(selected_id == Some(state.id()))
                .into()
        });
        let grid = Grid::with_children(rows)
            .columns(columns)
            .spacing(spacing::SM)
            .height(Length::Shrink);

        container(
            container(grid)
                .width(Length::Fill)
                .max_width(BOTTLE_LIST_MAX_WIDTH),
        )
        .center_x(Length::Fill)
        .into()
    })
    .height(Length::Shrink)
    .into()
}

pub fn creation(state: &State) -> Element<'_, Message> {
    let creating = state.is_creating();
    let name = TextRow::new("Bottle Name", state.bottle_name()).icon(Icon::Person);
    let name = if creating {
        name
    } else {
        name.on_input(Message::BottleNameChanged)
    };
    let runner =
        SelectorRow::new("Runner", state.runners(), state.selected_runner()).icon(Icon::Run);
    let runner = if creating {
        runner
    } else {
        runner.on_selected(Message::RunnerSelected)
    };
    let content = column![
        name,
        runner,
        SelectorRow::new("Purpose", &PURPOSES, Some(&PURPOSES[0])),
        SelectorRow::new("Architecture", &ARCHITECTURES, Some(&ARCHITECTURES[0])).icon(Icon::Chip),
        PickerRow::new("Use Recipe").description("Choose the location"),
    ]
    .spacing(12);

    if let Some(error) = state.creation_error() {
        column![
            InfoCard::new(InfoCardKind::Error, "Could not create bottle", error)
                .width(Length::Fill),
            content,
        ]
        .spacing(12)
        .into()
    } else {
        content.into()
    }
}

pub fn status(state: &BottleState) -> Element<'_, Message> {
    LogPanel::new("Win64", state.runner().name(), "Stopped", Icon::Power).into()
}

pub fn programs<'a>(
    feature: &'a State,
    bottle: Bottle,
    bottle_state: &'a BottleState,
) -> Element<'a, Message> {
    let programs = bottle_state.programs().collect::<Vec<_>>();
    let items = std::iter::once(new_program_target().into()).chain(
        programs
            .iter()
            .copied()
            .map(|program| program_card(bottle.clone(), program)),
    );

    let grid = Grid::with_children(items)
        .fluid(400.0)
        .spacing(spacing::MD)
        .height(Length::Shrink);

    if let Some(error) = feature.launch_error() {
        column![
            InfoCard::new(InfoCardKind::Error, "Program launch failed", error).width(Length::Fill),
            grid,
        ]
        .spacing(12)
        .into()
    } else {
        grid.into()
    }
}

fn new_program_target<'a>() -> ActionTile<'a, Message> {
    const ICON_CONTAINER_SIZE: f32 = 44.0;

    let icon = container(Icon::Plus.view().width(16).height(16))
        .width(ICON_CONTAINER_SIZE)
        .height(ICON_CONTAINER_SIZE)
        .align_x(Center)
        .align_y(Center)
        .style(|theme: &Theme| {
            container::Style::default()
                .background(crate::theme::colors(theme).surface)
                .border(iced::Border::default().rounded(ICON_CONTAINER_SIZE / 2.0))
        });
    let labels = column![
        text("New Program").size(17).medium(),
        text("Install or add a program.").size(14),
    ]
    .spacing(spacing::XS);

    let content = container(row![icon, labels].spacing(16).align_y(Center)).center_x(Length::Fill);

    ActionTile::disabled(content)
        .width(Length::Fill)
        .padding([72.0, spacing::LG])
}

fn program_card(bottle: Bottle, program: &bottles_core::Program) -> Element<'_, Message> {
    ArtworkCard::new(program.name(), "Installed program")
        .menu(CardAction::new("More actions", Icon::EllipsisVertical))
        .primary(
            CardAction::new("Play", Icon::Play).on_press(Message::LaunchProgram {
                bottle,
                program_id: program.id(),
            }),
        )
        .into()
}
