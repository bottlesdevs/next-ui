//! Bottles: the list of bottles, the new-bottle creation flow, and the
//! "Programs" detail tab.

use std::sync::Arc;

use bottles_core::{Addons, Bottle, BottleManager, BottleState, Slot, Storage};
use iced::{
    Center, Element, Length, Task, Theme,
    widget::{Grid, column, container, responsive, row, text},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    icons::Icon,
    operation,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        artwork_card::{ArtworkCard, CardAction},
        drop_target::DropTarget,
        info_card::{InfoCard, Kind as InfoCardKind},
        list_row::ListRow,
        picker_row::PickerRow,
        selector_row::SelectorRow,
        spacing,
        status_bar::{BottleStatus, StatusBar},
        text::TextExt as _,
        text_row::TextRow,
    },
};

const BOTTLE_LIST_MAX_WIDTH: f32 = 720.0;
const BOTTLE_TRACK_MIN_WIDTH: f32 = 300.0;

const PURPOSES: [&str; 4] = ["Gaming", "Software", "Gaming (ULWGL)", "Custom"];
const ARCHITECTURES: [&str; 2] = ["Win64", "Win32"];

#[derive(Clone, PartialEq)]
pub struct RunnerOption {
    id: Uuid,
    label: String,
}

impl std::fmt::Display for RunnerOption {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub fn bottle_events(
    manager: &BottleManager,
) -> impl iced::futures::Stream<Item = Vec<Bottle>> + Send + 'static + use<> {
    manager.watch()
}

pub fn bottle_state_events(
    bottle: &Bottle,
) -> impl iced::futures::Stream<Item = Arc<BottleState>> + Send + 'static + use<> {
    bottle.watch()
}

#[derive(Clone)]
pub enum Message {
    CreateBottle,
    BottleCreation(operation::Event<u64, Bottle>),
    BottleNameChanged(String),
    RunnerSelected(RunnerOption),
    PurposeSelected(&'static str),
    ArchitectureSelected(&'static str),
    LaunchProgram { bottle: Bottle, program_id: Uuid },
    ProgramLaunched(Result<u32, Arc<bottles_core::error::Error>>),
    Noop,
}

pub enum Output {
    Created,
}

pub struct State {
    manager: BottleManager,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
    purpose: &'static str,
    architecture: &'static str,
    creation_generation: u64,
    creation_cancellation: Option<CancellationToken>,
    program_launches: usize,
    last_error: Option<String>,
}

impl State {
    pub fn new(manager: BottleManager, addons: &Addons) -> Self {
        let runners = addons
            .components()
            .iter()
            .filter(|entry| entry.slot() == Slot::Runner)
            .map(|entry| RunnerOption {
                id: entry.id(),
                label: format!("{} {}", entry.name(), entry.version()),
            })
            .collect::<Vec<_>>();
        let selected_runner = runners.first().cloned();
        Self {
            manager,
            bottle_name: "Gaming paradise".into(),
            runners,
            selected_runner,
            purpose: PURPOSES[0],
            architecture: ARCHITECTURES[0],
            creation_generation: 0,
            creation_cancellation: None,
            program_launches: 0,
            last_error: None,
        }
    }

    pub(super) fn manager(&self) -> &BottleManager {
        &self.manager
    }

    pub fn reset_creation(&mut self) {
        self.last_error = None;
    }

    pub fn cancel_creation(&self) {
        if let Some(cancellation) = &self.creation_cancellation {
            cancellation.cancel();
        }
    }

    pub fn has_active_operation(&self) -> bool {
        self.creation_cancellation.is_some() || self.program_launches > 0
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Output>) {
        let mut output = None;
        match message {
            Message::CreateBottle => {
                if self.creation_cancellation.is_none()
                    && let Some(runner) = self.selected_runner.clone()
                {
                    let name = self.bottle_name.clone();
                    let manager = self.manager.clone();
                    let operation = manager.create(name, Storage::Standard, runner.id);
                    self.creation_generation = self.creation_generation.wrapping_add(1);
                    let generation = self.creation_generation;
                    let (cancellation, task) = operation::run(operation, generation);
                    self.creation_cancellation = Some(cancellation);

                    self.last_error = None;

                    return (task.map(Message::BottleCreation), None);
                }
            }
            Message::BottleCreation(operation::Event::Finished { key, outcome })
                if key == self.creation_generation =>
            {
                self.creation_cancellation = None;
                match outcome {
                    operation::Outcome::Succeeded(_) => {
                        self.last_error = None;
                        output = Some(Output::Created);
                    }
                    operation::Outcome::Cancelled => {
                        self.last_error = Some("Bottle creation was cancelled.".into());
                    }
                    operation::Outcome::Failed(error) => {
                        self.last_error = Some(error.to_string());
                    }
                }
            }
            Message::BottleNameChanged(name) => self.bottle_name = name,
            Message::RunnerSelected(runner) => self.selected_runner = Some(runner),
            Message::PurposeSelected(purpose) => self.purpose = purpose,
            Message::ArchitectureSelected(architecture) => self.architecture = architecture,
            Message::LaunchProgram { bottle, program_id } => {
                self.program_launches += 1;
                return (
                    Task::perform(
                        async move { bottle.launch_program(program_id).await.map_err(Arc::new) },
                        Message::ProgramLaunched,
                    ),
                    None,
                );
            }
            Message::ProgramLaunched(Err(error)) => {
                self.program_launches = self.program_launches.saturating_sub(1);
                self.last_error = Some(error.to_string());
            }
            Message::ProgramLaunched(Ok(_)) => {
                self.program_launches = self.program_launches.saturating_sub(1);
                self.last_error = None;
            }
            Message::BottleCreation(_) | Message::Noop => {}
        }

        (Task::none(), output)
    }

    pub fn rows_view<'a, Msg: 'static + Clone>(
        &self,
        bottle_states: &'a [Arc<BottleState>],
        selected_id: Option<Uuid>,
        on_select: impl Fn(Uuid) -> Msg + 'a,
    ) -> Element<'a, Msg> {
        responsive(move |size| {
            let columns = usize::from(size.width >= BOTTLE_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
            let rows = bottle_states.iter().map(|state| {
                let row: ListRow<'_, Msg> =
                    ActionRow::new(state.name(), ActionRowState::Ready(on_select(state.id())))
                        .description(state.runner().name())
                        .icon(Icon::Bottles)
                        .into();

                row.selected(selected_id == Some(state.id())).into()
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

    pub fn creation_view(&self) -> Element<'_, Message> {
        let content = column![
            TextRow::new("Bottle Name", &self.bottle_name)
                .icon(Icon::Person)
                .on_input(Message::BottleNameChanged),
            SelectorRow::new("Runner", &self.runners, self.selected_runner.as_ref())
                .icon(Icon::Run)
                .on_selected(Message::RunnerSelected),
            SelectorRow::new("Purpose", &PURPOSES, Some(&self.purpose))
                .on_selected(Message::PurposeSelected),
            SelectorRow::new("Architecture", &ARCHITECTURES, Some(&self.architecture),)
                .icon(Icon::Chip)
                .on_selected(Message::ArchitectureSelected),
            PickerRow::new("Use Recipe")
                .description("Choose the location")
                .on_press(Message::Noop),
        ]
        .spacing(12);

        if let Some(error) = &self.last_error {
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

    pub fn bottle_status<'a>(&self, state: &'a BottleState) -> Element<'a, Message> {
        StatusBar::new("Win64", state.runner().name(), BottleStatus::Stopped).into()
    }

    pub fn program_grid<'a>(&self, bottle: Bottle, state: &'a BottleState) -> Element<'a, Message> {
        let programs = state.programs().collect::<Vec<_>>();
        let items = std::iter::once(new_program_target().into()).chain(
            programs
                .iter()
                .copied()
                .map(|program| program_card(bottle.clone(), program)),
        );

        Grid::with_children(items)
            .fluid(400.0)
            .spacing(spacing::MD)
            .height(Length::Shrink)
            .into()
    }
}

fn new_program_target<'a>() -> DropTarget<'a, Message> {
    const ICON_CONTAINER_SIZE: f32 = 44.0;

    let icon = container(Icon::Plus.view().width(16).height(16))
        .width(ICON_CONTAINER_SIZE)
        .height(ICON_CONTAINER_SIZE)
        .align_x(Center)
        .align_y(Center)
        .style(|theme: &Theme| {
            container::Style::default()
                .background(theme.extended_palette().background.weak.color)
                .border(iced::Border::default().rounded(ICON_CONTAINER_SIZE / 2.0))
        });
    let labels = column![
        text("New Program").size(17).medium(),
        text("Install or add a program.").size(14),
    ]
    .spacing(spacing::XS);

    let content = container(row![icon, labels].spacing(16).align_y(Center)).center_x(Length::Fill);

    DropTarget::new(content, Message::Noop)
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
