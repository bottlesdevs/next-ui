//! Bottles: the list of bottles, the new-bottle creation flow, and the
//! "Programs" detail tab.

use std::sync::Arc;

use bottles_core::{Addons, Bottle, BottleManager, BottleState, Progress, Slot, Storage};
use iced::{
    Element, Fill, Task,
    widget::{Column, column, container, image, row},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    icons::Icon,
    operation,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        artwork_card::{ArtworkCard, CardAction},
        picker_row::PickerRow,
        row_group::RowGroup,
        selector_row::SelectorRow,
        split_view::PaneMode,
        status_bar::{StatusBar, StatusState},
        text_row::TextRow,
    },
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

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
    ToggleCreationLog,
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
    creation_log: String,
    creation_log_expanded: bool,
    creation_failed: bool,
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
            creation_log: String::new(),
            creation_log_expanded: false,
            creation_failed: false,
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
        self.creation_log.clear();
        self.creation_failed = false;
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

                    self.creation_log.clear();
                    self.creation_failed = false;
                    self.creation_log_expanded = true;
                    self.last_error = None;

                    return (task.map(Message::BottleCreation), None);
                }
            }
            Message::BottleCreation(operation::Event::Progress { key, progress })
                if key == self.creation_generation =>
            {
                if !self.creation_log.is_empty() {
                    self.creation_log.push('\n');
                }
                self.creation_log.push_str(&progress_log_line(&progress));
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
                        self.creation_failed = true;
                        self.last_error = Some("Bottle creation was cancelled.".into());
                    }
                    operation::Outcome::Failed(error) => {
                        self.creation_failed = true;
                        self.last_error = Some(error.to_string());
                    }
                }

                if let Some(error) = &self.last_error {
                    if !self.creation_log.is_empty() {
                        self.creation_log.push('\n');
                    }
                    self.creation_log
                        .push_str(&format!("{} Failed: {error}", timestamp()));
                }
            }
            Message::ToggleCreationLog => {
                self.creation_log_expanded = !self.creation_log_expanded;
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
        width: f32,
        mode: PaneMode,
        on_select: impl Fn(Uuid) -> Msg,
    ) -> Element<'a, Msg> {
        let columns = usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = bottle_states
            .iter()
            .fold(RowGroup::new().columns(columns), |rows, state| {
                rows.add(
                    ActionRow::new(state.name(), ActionRowState::Ready(on_select(state.id())))
                        .description(state.runner().name())
                        .icon(Icon::Bottles),
                )
            });

        container(rows).max_width(1150).into()
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

        let mut page = column![content].width(Fill);

        if !self.creation_log.is_empty() {
            let state = if self.creation_failed {
                StatusState::Failed
            } else {
                StatusState::Starting
            };

            page = page.push(
                StatusBar::new(
                    self.architecture,
                    self.selected_runner
                        .as_ref()
                        .map(|runner| runner.label.as_str())
                        .unwrap_or_default(),
                    state,
                )
                .log(&self.creation_log)
                .expanded(self.creation_log_expanded)
                .on_toggle(Message::ToggleCreationLog),
            );
        }

        page.into()
    }

    pub fn program_grid<'a>(
        &self,
        bottle: Bottle,
        state: &'a BottleState,
        width: f32,
    ) -> Element<'a, Message> {
        let programs = state.programs().collect::<Vec<_>>();

        if width >= CONTENT_GRID_BREAKPOINT {
            Column::with_children(programs.chunks(2).map(|chunk| {
                row(chunk
                    .iter()
                    .copied()
                    .map(|program| program_card(bottle.clone(), program)))
                .spacing(12)
                .into()
            }))
            .spacing(12)
            .into()
        } else {
            Column::with_children(
                programs
                    .iter()
                    .copied()
                    .map(|program| program_card(bottle.clone(), program)),
            )
            .spacing(12)
            .into()
        }
    }
}

fn program_card(bottle: Bottle, program: &bottles_core::Program) -> Element<'_, Message> {
    ArtworkCard::new(program.name(), program.executable())
        .secondary(CardAction::new("Settings", Icon::Gear).on_press(Message::Noop))
        .primary(
            CardAction::new("Play", Icon::Play).on_press(Message::LaunchProgram {
                bottle,
                program_id: program.id(),
            }),
        )
        .banner(sample_image(program.id()))
        .into()
}

fn sample_image(id: Uuid) -> image::Handle {
    let seed = id.as_bytes()[0] % 16;
    let first = [45 + seed * 5, 50 + seed * 3, 65 + seed * 4];
    let second = [first[2], first[0] + 20, first[1] + 10];

    image::Handle::from_rgba(
        2,
        2,
        vec![
            first[0], first[1], first[2], 255, second[0], second[1], second[2], 255, second[0],
            second[1], second[2], 255, first[0], first[1], first[2], 255,
        ],
    )
}

pub fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!(
        "{:02}:{:02}:{:02}",
        (seconds / 3600) % 24,
        (seconds / 60) % 60,
        seconds % 60
    )
}

pub fn progress_log_line(progress: &Progress) -> String {
    match progress.transfer.as_ref().and_then(|_| progress.fraction()) {
        Some(fraction) => format!(
            "{} {} ({:.0}%)",
            timestamp(),
            progress.stage,
            fraction * 100.0
        ),
        None => format!("{} {}", timestamp(), progress.stage),
    }
}
