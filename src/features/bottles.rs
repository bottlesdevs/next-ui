use std::sync::Arc;

use bottles_core::{Addons, Bottle, BottleManager, BottleState, Slot, Storage};
use iced::Task;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::operation;

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
    LaunchProgram { bottle: Bottle, program_id: Uuid },
    ProgramLaunched(Result<u32, Arc<bottles_core::error::Error>>),
}

pub enum Output {
    Created,
}

pub struct State {
    manager: BottleManager,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
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
            creation_generation: 0,
            creation_cancellation: None,
            program_launches: 0,
            last_error: None,
        }
    }

    pub fn manager(&self) -> &BottleManager {
        &self.manager
    }

    pub fn bottle_name(&self) -> &str {
        &self.bottle_name
    }

    pub fn runners(&self) -> &[RunnerOption] {
        &self.runners
    }

    pub fn selected_runner(&self) -> Option<&RunnerOption> {
        self.selected_runner.as_ref()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
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

    pub fn is_creating(&self) -> bool {
        self.creation_cancellation.is_some()
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
            Message::BottleCreation(_) => {}
        }

        (Task::none(), output)
    }
}
