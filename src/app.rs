use iced::{Element, Subscription, Task, Theme};
use next_config::Config;
use serde::{Deserialize, Serialize};

use crate::shell;

const ONBOARDING_CONFIG_FILE: &str = "onboarding.toml";

#[derive(Debug, Default, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
struct OnboardingConfig {
    completed: bool,
}

fn onboarding_config_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "usebottles", "bottles-next")
        .map(|dirs| dirs.config_dir().join(ONBOARDING_CONFIG_FILE))
}

fn has_onboarded() -> bool {
    let Some(path) = onboarding_config_path() else {
        return false;
    };

    futures_lite::future::block_on(next_config::load::<OnboardingConfig>(&path))
        .map(|config| config.completed)
        .unwrap_or(false)
}

fn mark_onboarded<T: Send + 'static>() -> Task<T> {
    Task::future(async {
        let Some(path) = onboarding_config_path() else {
            return;
        };
        let _ = next_config::save(&path, &OnboardingConfig { completed: true }).await;
    })
    .discard()
}

pub enum App {
    Onboarding(Box<crate::features::onboarding::State>),
    Main(Box<shell::State>),
}

#[derive(Clone)]
pub enum AppMessage {
    Onboarding(crate::features::onboarding::Message),
    Main(Box<shell::Message>),
}

impl App {
    pub fn new() -> (Self, Task<AppMessage>) {
        if has_onboarded() {
            let (example, task) = shell::State::new();

            return (
                Self::Main(Box::new(example)),
                task.map(|message| AppMessage::Main(Box::new(message))),
            );
        }

        let (state, task) = crate::features::onboarding::State::new();

        (
            Self::Onboarding(Box::new(state)),
            task.map(AppMessage::Onboarding),
        )
    }

    pub fn theme(&self) -> Theme {
        match self {
            Self::Onboarding(state) => state.theme(),
            Self::Main(example) => example.theme(),
        }
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        match self {
            Self::Onboarding(state) => state.subscription().map(AppMessage::Onboarding),
            Self::Main(example) => example
                .subscription()
                .map(|message| AppMessage::Main(Box::new(message))),
        }
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Onboarding(message) => {
                let Self::Onboarding(state) = self else {
                    return Task::none();
                };

                if let crate::features::onboarding::Message::Finished = message {
                    let (example, task) = match state.take_bottles() {
                        Some(bottles) => shell::State::new_with_bottles(bottles),
                        None => shell::State::new(),
                    };
                    *self = Self::Main(Box::new(example));
                    return Task::batch([
                        task.map(|message| AppMessage::Main(Box::new(message))),
                        mark_onboarded(),
                    ]);
                }

                state.update(message).map(AppMessage::Onboarding)
            }
            AppMessage::Main(message) => {
                let Self::Main(example) = self else {
                    return Task::none();
                };

                example
                    .update(*message)
                    .map(|message| AppMessage::Main(Box::new(message)))
            }
        }
    }

    pub fn view(&self) -> Element<'_, AppMessage> {
        match self {
            Self::Onboarding(state) => state.view().map(AppMessage::Onboarding),
            Self::Main(example) => example
                .view()
                .map(|message| AppMessage::Main(Box::new(message))),
        }
    }
}
