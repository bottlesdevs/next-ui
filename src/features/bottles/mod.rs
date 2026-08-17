//! Bottles: the list of bottles, the new-bottle creation flow, and the
//! "Programs" detail tab. Owns the `Bottles` handle and everything derived
//! from it (bottle list, per-bottle state, available runners).

use std::sync::Arc;

use bottles_core::{Bottle, BottleManager, BottleState, Bottles, Config as CoreConfig, Progress, Slot, Storage};
use iced::{
    Element, Fill, Task,
    widget::{Column, column, container, image, row},
};
use uuid::Uuid;

use crate::{
    components::{
        action_row::{ActionRow, State as ActionRowState},
        artwork_card::{ArtworkCard, CardAction},
        picker_row::PickerRow,
        row_group::RowGroup,
        selector_row::SelectorRow,
        split_view::PaneMode,
        status_bar::{StatusBar, StatusState},
        text_row::TextRow,
    },
    icons::Icon,
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

#[derive(Clone)]
pub struct BottleManagerHandle(pub BottleManager);

impl std::hash::Hash for BottleManagerHandle {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

pub fn bottle_events(
    handle: &BottleManagerHandle,
) -> std::pin::Pin<Box<dyn iced::futures::Stream<Item = Message> + Send>> {
    use iced::futures::StreamExt as _;

    let manager = handle.0.clone();

    Box::pin(manager.watch().map(Message::BottleListChanged))
}

#[derive(Clone)]
pub enum Message {
    AddBottle,
    CreateBottle,
    BottleCreationProgress(Progress),
    BottleCreated(Result<Bottle, String>),
    ToggleCreationLog,
    BottleNameChanged(String),
    RunnerSelected(RunnerOption),
    PurposeSelected(&'static str),
    ArchitectureSelected(&'static str),
    BottlesLoaded(Result<Arc<Bottles>, String>),
    BottleListChanged(Vec<Bottle>),
    LaunchProgram(Uuid),
    ProgramLaunched(Result<u32, String>),
    Noop,
}

pub struct State {
    bottles: Option<Bottles>,
    bottle_list: Vec<Bottle>,
    bottle_states: Vec<Arc<BottleState>>,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
    purpose: &'static str,
    architecture: &'static str,
    creation_log: String,
    creation_log_expanded: bool,
    creation_failed: bool,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            bottles: None,
            bottle_list: Vec::new(),
            bottle_states: Vec::new(),
            bottle_name: "Gaming paradise".into(),
            runners: Vec::new(),
            selected_runner: None,
            purpose: PURPOSES[0],
            architecture: ARCHITECTURES[0],
            creation_log: String::new(),
            creation_log_expanded: false,
            creation_failed: false,
        }
    }

    pub fn boot() -> Task<Message> {
        Task::perform(
            async {
                Bottles::open(CoreConfig::default())
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::BottlesLoaded,
        )
    }

    pub fn new_with_bottles(bottles: Bottles) -> Self {
        let mut state = Self::new();
        state.apply_bottles(bottles);
        state
    }

    fn apply_bottles(&mut self, bottles: Bottles) {
        self.bottle_list = bottles.bottles().list();
        self.runners = bottles
            .addons()
            .components()
            .iter()
            .filter(|entry| entry.slot() == Slot::Runner)
            .map(|entry| RunnerOption {
                id: entry.id(),
                label: format!("{} {}", entry.name(), entry.version()),
            })
            .collect();
        self.selected_runner = self.runners.first().cloned();
        self.bottles = Some(bottles);
        self.refresh_states();
    }

    pub fn refresh_states(&mut self) {
        self.bottle_states = self
            .bottle_list
            .iter()
            .filter_map(|bottle| bottle.state().ok())
            .collect();
    }

    pub fn states(&self) -> &[Arc<BottleState>] {
        &self.bottle_states
    }

    pub fn bottle_handle(&self, id: Uuid) -> Option<Bottle> {
        self.bottle_list
            .iter()
            .find(|bottle| bottle.state().is_ok_and(|state| state.id() == id))
            .cloned()
    }

    pub fn bottle_state(&self, id: Uuid) -> Option<&Arc<BottleState>> {
        self.bottle_states.iter().find(|state| state.id() == id)
    }

    pub fn bottle_manager_handle(&self) -> Option<BottleManagerHandle> {
        self.bottles
            .as_ref()
            .map(|bottles| BottleManagerHandle(bottles.bottles().clone()))
    }

    pub fn reset_creation(&mut self) {
        self.creation_log.clear();
        self.creation_failed = false;
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddBottle => self.reset_creation(),
            Message::CreateBottle => {
                if let (Some(bottles), Some(runner)) = (&self.bottles, self.selected_runner.clone())
                {
                    let name = self.bottle_name.clone();
                    let manager = bottles.bottles().clone();
                    let operation = manager.create(name, Storage::Standard, runner.id);
                    let progress = operation.progress();

                    self.creation_log.clear();
                    self.creation_failed = false;
                    self.creation_log_expanded = true;

                    let progress_task = Task::stream(progress).map(Message::BottleCreationProgress);
                    let result_task = Task::perform(
                        async move { operation.await.map_err(|err| err.to_string()) },
                        Message::BottleCreated,
                    );

                    return Task::batch([progress_task, result_task]);
                }
            }
            Message::BottleCreationProgress(progress) => {
                if !self.creation_log.is_empty() {
                    self.creation_log.push('\n');
                }
                self.creation_log.push_str(&progress_log_line(&progress));
            }
            Message::BottleCreated(Ok(_)) => {}
            Message::BottleCreated(Err(err)) => {
                self.creation_failed = true;

                if !self.creation_log.is_empty() {
                    self.creation_log.push('\n');
                }
                self.creation_log
                    .push_str(&format!("{} Failed: {err}", timestamp()));

                eprintln!("failed to create bottle: {err}");
            }
            Message::ToggleCreationLog => {
                self.creation_log_expanded = !self.creation_log_expanded;
            }
            Message::BottleNameChanged(name) => self.bottle_name = name,
            Message::RunnerSelected(runner) => self.selected_runner = Some(runner),
            Message::PurposeSelected(purpose) => self.purpose = purpose,
            Message::ArchitectureSelected(architecture) => self.architecture = architecture,
            Message::BottlesLoaded(Ok(bottles)) => {
                if let Ok(bottles) = Arc::try_unwrap(bottles) {
                    self.apply_bottles(bottles);
                }
            }
            Message::BottlesLoaded(Err(err)) => eprintln!("failed to open Bottles: {err}"),
            Message::BottleListChanged(list) => {
                self.bottle_list = list;
                self.refresh_states();
            }
            // `LaunchProgram` needs to know which bottle is currently
            // selected, which is shell-level navigation state (`split_view_state`)
            // that this feature doesn't own. The shell intercepts this
            // variant before it reaches `update` and calls
            // `launch_program` directly with the resolved bottle handle.
            Message::LaunchProgram(_) => {}
            Message::ProgramLaunched(Err(err)) => eprintln!("failed to launch program: {err}"),
            Message::ProgramLaunched(Ok(_)) | Message::Noop => {}
        }

        Task::none()
    }

    pub fn launch_program(&self, bottle: Bottle, program_id: Uuid) -> Task<Message> {
        Task::perform(
            async move { bottle.run(program_id).await.map_err(|err| err.to_string()) },
            Message::ProgramLaunched,
        )
    }

    pub fn rows_view<Msg: 'static + Clone>(
        &self,
        width: f32,
        mode: PaneMode,
        on_select: impl Fn(Uuid) -> Msg,
    ) -> Element<'_, Msg> {
        let columns = usize::from(mode == PaneMode::Single && width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self
            .bottle_states
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

    pub fn program_grid(&self, id: Uuid, width: f32) -> Element<'_, Message> {
        let programs = self
            .bottle_states
            .iter()
            .find(|state| state.id() == id)
            .map(|state| state.programs())
            .unwrap_or_default();

        if width >= CONTENT_GRID_BREAKPOINT {
            Column::with_children(
                programs
                    .chunks(2)
                    .map(|chunk| row(chunk.iter().map(program_card)).spacing(12).into()),
            )
            .spacing(12)
            .into()
        } else {
            Column::with_children(programs.iter().map(program_card))
                .spacing(12)
                .into()
        }
    }
}

fn program_card(program: &bottles_core::Program) -> Element<'_, Message> {
    ArtworkCard::new(&program.name, &program.executable)
        .secondary(CardAction::new("Settings", Icon::Gear).on_press(Message::Noop))
        .primary(CardAction::new("Play", Icon::Play).on_press(Message::LaunchProgram(program.id)))
        .banner(sample_image(program.id))
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
