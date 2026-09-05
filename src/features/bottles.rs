use std::sync::Arc;

use bottles_core::{Addons, Bottle, BottleManager, Slot, Storage};
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
    creation: Creation,
    launches: ProgramLaunches,
}

struct Creation {
    manager: BottleManager,
    bottle_name: String,
    runners: Vec<RunnerOption>,
    selected_runner: Option<RunnerOption>,
    creation_generation: u64,
    creation_cancellation: Option<CancellationToken>,
    last_error: Option<String>,
}

#[derive(Default)]
struct ProgramLaunches {
    active: usize,
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
            creation: Creation {
                manager,
                bottle_name: "Gaming paradise".into(),
                runners,
                selected_runner,
                creation_generation: 0,
                creation_cancellation: None,
                last_error: None,
            },
            launches: ProgramLaunches::default(),
        }
    }

    pub fn bottle_name(&self) -> &str {
        &self.creation.bottle_name
    }

    pub fn runners(&self) -> &[RunnerOption] {
        &self.creation.runners
    }

    pub fn selected_runner(&self) -> Option<&RunnerOption> {
        self.creation.selected_runner.as_ref()
    }

    pub fn creation_error(&self) -> Option<&str> {
        self.creation.last_error.as_deref()
    }

    pub fn launch_error(&self) -> Option<&str> {
        self.launches.last_error.as_deref()
    }

    pub fn reset_creation(&mut self) {
        self.creation.last_error = None;
    }

    pub fn cancel_creation(&self) {
        if let Some(cancellation) = &self.creation.creation_cancellation {
            cancellation.cancel();
        }
    }

    pub fn has_active_operation(&self) -> bool {
        self.creation.creation_cancellation.is_some() || self.launches.active > 0
    }

    pub fn is_creating(&self) -> bool {
        self.creation.creation_cancellation.is_some()
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Output>) {
        let mut output = None;
        match message {
            Message::CreateBottle => {
                if self.creation.creation_cancellation.is_none()
                    && let Some(runner) = self.creation.selected_runner.clone()
                {
                    let name = self.creation.bottle_name.clone();
                    let manager = self.creation.manager.clone();
                    let operation = manager.create(name, Storage::Standard, runner.id);
                    self.creation.creation_generation =
                        self.creation.creation_generation.wrapping_add(1);
                    let generation = self.creation.creation_generation;
                    let (cancellation, task) = operation::run(operation, generation);
                    self.creation.creation_cancellation = Some(cancellation);
                    self.creation.last_error = None;

                    return (task.map(Message::BottleCreation), None);
                }
            }
            Message::BottleCreation(operation::Event::Finished { key, outcome })
                if key == self.creation.creation_generation =>
            {
                self.creation.creation_cancellation = None;
                match outcome {
                    operation::Outcome::Succeeded(_) => {
                        self.creation.last_error = None;
                        output = Some(Output::Created);
                    }
                    operation::Outcome::Cancelled => {
                        self.creation.last_error = Some("Bottle creation was cancelled.".into());
                    }
                    operation::Outcome::Failed(error) => {
                        self.creation.last_error = Some(error.to_string());
                    }
                }
            }
            Message::BottleNameChanged(name) => self.creation.bottle_name = name,
            Message::RunnerSelected(runner) => self.creation.selected_runner = Some(runner),
            Message::LaunchProgram { bottle, program_id } => {
                self.launches.active += 1;
                return (
                    Task::perform(
                        async move { bottle.launch_program(program_id).await.map_err(Arc::new) },
                        Message::ProgramLaunched,
                    ),
                    None,
                );
            }
            Message::ProgramLaunched(Err(error)) => {
                self.launches.active = self.launches.active.saturating_sub(1);
                self.launches.last_error = Some(error.to_string());
            }
            Message::ProgramLaunched(Ok(_)) => {
                self.launches.active = self.launches.active.saturating_sub(1);
                self.launches.last_error = None;
            }
            Message::BottleCreation(_) => {}
        }

        (Task::none(), output)
    }
}
