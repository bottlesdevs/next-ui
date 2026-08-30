use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use bottles_core::{Bottles, Config as CoreConfig, error::Error as CoreError};
use iced::{Element, Fill, Subscription, Task, Theme, theme::Mode as ThemeMode};
use next_config::Config;
use serde::{Deserialize, Serialize};

use crate::{
    classic, onboarding, theme,
    ui::chrome,
    widgets::{
        button::{Button, ButtonKind},
        dialog::WindowModal,
        header_bar::HeaderBar,
    },
};

const APP_CONFIG_FILE: &str = "config.toml";
const COMPONENT_CATALOG_URL: &str = "https://bottles-next-deps.bromb.in/api/v1/catalog/components";
const DEPENDENCY_CATALOG_URL: &str =
    "https://bottles-next-deps.bromb.in/api/v1/catalog/dependencies";

#[derive(Clone, Debug)]
pub(crate) enum AppError {
    ConfigDirectory,
    Config {
        action: &'static str,
        path: PathBuf,
        source: Arc<next_config::error::Error>,
    },
    Core(Arc<CoreError>),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDirectory => {
                formatter.write_str("could not resolve the application config directory")
            }
            Self::Config {
                action,
                path,
                source,
            } => write!(formatter, "failed to {action} {}: {source}", path.display()),
            Self::Core(error) => write!(formatter, "Bottles core failed: {error}"),
        }
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
pub struct AppConfig {
    #[serde(default)]
    pub experience: Option<Experience>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Experience {
    Next,
    Classic,
}

pub(crate) struct App {
    phase: Phase,
    theme: Theme,
}

enum Phase {
    Booting,
    Onboarding {
        core: Arc<Bottles>,
        state: Box<onboarding::State>,
        saving: Option<Experience>,
        notice: Option<AppError>,
    },
    Workspace {
        core: Arc<Bottles>,
        workspace: Workspace,
        transition: WorkspaceTransition,
        notice: Option<AppError>,
    },
    ShuttingDown,
    Failed(AppError),
}

enum Workspace {
    Classic(Box<classic::State>),
    Unavailable(Experience),
}

impl Workspace {
    fn experience(&self) -> Experience {
        match self {
            Self::Classic(state) => state.experience(),
            Self::Unavailable(experience) => *experience,
        }
    }

    fn has_active_operations(&self) -> bool {
        match self {
            Self::Classic(state) => state.has_active_operations(),
            Self::Unavailable(_) => false,
        }
    }

    fn cancel_active_operations(&mut self) {
        if let Self::Classic(state) = self {
            state.cancel_active_operations();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceTransition {
    Ready,
    Confirming(Experience),
    Draining(Experience),
    Saving(Experience),
}

#[derive(Clone)]
pub(crate) enum AppMessage {
    Booted(AppResult<Boot>),
    Onboarding(onboarding::Message),
    Workspace(WorkspaceMessage),
    RequestExperience(Experience),
    ExperienceSaved {
        experience: Experience,
        result: AppResult<()>,
    },
    ConfirmExperienceSwitch,
    CancelExperienceSwitch,
    DismissNotice,
    CloseRequested,
    ShutdownFinished(AppResult<()>),
    Window(chrome::Action),
    SystemThemeChanged(ThemeMode),
}

impl std::fmt::Debug for AppMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Booted(_) => "Booted",
            Self::Onboarding(_) => "Onboarding",
            Self::Workspace(_) => "Workspace",
            Self::RequestExperience(_) => "RequestExperience",
            Self::ExperienceSaved { .. } => "ExperienceSaved",
            Self::ConfirmExperienceSwitch => "ConfirmExperienceSwitch",
            Self::CancelExperienceSwitch => "CancelExperienceSwitch",
            Self::DismissNotice => "DismissNotice",
            Self::CloseRequested => "CloseRequested",
            Self::ShutdownFinished(_) => "ShutdownFinished",
            Self::Window(_) => "Window",
            Self::SystemThemeChanged(_) => "SystemThemeChanged",
        })
    }
}

#[derive(Clone)]
pub(crate) enum WorkspaceMessage {
    Classic(Box<classic::Message>),
}

#[derive(Clone)]
pub(crate) struct Boot {
    config: AppConfig,
    core: Arc<Bottles>,
}

impl App {
    pub(crate) fn new() -> (Self, Task<AppMessage>) {
        (
            Self {
                phase: Phase::Booting,
                theme: theme::for_mode(ThemeMode::default()),
            },
            Task::batch([
                Task::perform(boot(), AppMessage::Booted),
                iced::system::theme().map(AppMessage::SystemThemeChanged),
            ]),
        )
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub(crate) fn subscription(&self) -> Subscription<AppMessage> {
        let phase = match &self.phase {
            Phase::Workspace {
                workspace: Workspace::Classic(state),
                ..
            } => state
                .subscription()
                .map(|message| AppMessage::Workspace(WorkspaceMessage::Classic(Box::new(message)))),
            Phase::Booting
            | Phase::Onboarding { .. }
            | Phase::Workspace {
                workspace: Workspace::Unavailable(_),
                ..
            }
            | Phase::ShuttingDown
            | Phase::Failed(_) => Subscription::none(),
        };

        Subscription::batch([
            phase,
            iced::system::theme_changes().map(AppMessage::SystemThemeChanged),
            iced::event::listen().filter_map(|event| {
                matches!(
                    event,
                    iced::Event::Window(iced::window::Event::CloseRequested)
                )
                .then_some(AppMessage::CloseRequested)
            }),
        ])
    }

    pub(crate) fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Booted(result) if matches!(self.phase, Phase::ShuttingDown) => {
                return match result {
                    Ok(boot) => self.shutdown(boot.core),
                    Err(_) => iced::exit(),
                };
            }
            AppMessage::Booted(Ok(boot)) if matches!(self.phase, Phase::Booting) => {
                return self.finish_boot(boot);
            }
            AppMessage::Booted(Err(error)) if matches!(self.phase, Phase::Booting) => {
                self.phase = Phase::Failed(error);
            }
            AppMessage::Booted(_) => {}
            AppMessage::Onboarding(onboarding::Message::Finished(experience)) => {
                if !matches!(self.phase, Phase::Onboarding { .. }) {
                    return Task::none();
                }
                return self.request_experience(experience);
            }
            AppMessage::Onboarding(message) => {
                let Phase::Onboarding { state, .. } = &mut self.phase else {
                    return Task::none();
                };
                return state.update(message).map(AppMessage::Onboarding);
            }
            AppMessage::Workspace(WorkspaceMessage::Classic(message)) => {
                match message.as_ref() {
                    classic::Message::RequestExperience(experience) => {
                        return self.request_experience(*experience);
                    }
                    _ => {}
                }

                let task = {
                    let Phase::Workspace {
                        workspace: Workspace::Classic(state),
                        ..
                    } = &mut self.phase
                    else {
                        return Task::none();
                    };

                    state.update(*message).map(|message| {
                        AppMessage::Workspace(WorkspaceMessage::Classic(Box::new(message)))
                    })
                };

                return Task::batch([task, self.advance_experience_switch()]);
            }
            AppMessage::RequestExperience(experience) => {
                return self.request_experience(experience);
            }
            AppMessage::ExperienceSaved { experience, result } => {
                return self.finish_experience_save(experience, result);
            }
            AppMessage::ConfirmExperienceSwitch => return self.confirm_experience_switch(),
            AppMessage::CancelExperienceSwitch => {
                let Phase::Workspace { transition, .. } = &mut self.phase else {
                    return Task::none();
                };
                if matches!(transition, WorkspaceTransition::Confirming(_)) {
                    *transition = WorkspaceTransition::Ready;
                }
            }
            AppMessage::DismissNotice => match &mut self.phase {
                Phase::Onboarding { notice, .. } | Phase::Workspace { notice, .. } => {
                    *notice = None;
                }
                Phase::Booting | Phase::ShuttingDown | Phase::Failed(_) => {}
            },
            AppMessage::CloseRequested => return self.request_close(),
            AppMessage::ShutdownFinished(result) => {
                if !matches!(self.phase, Phase::ShuttingDown) {
                    return Task::none();
                }
                let _ = result;
                return iced::exit();
            }
            AppMessage::Window(action) => {
                return action.task().unwrap_or_else(|| self.request_close());
            }
            AppMessage::SystemThemeChanged(mode) => self.theme = theme::for_mode(mode),
        }

        Task::none()
    }

    pub(crate) fn view(&self) -> Element<'_, AppMessage> {
        let body = match &self.phase {
            Phase::Booting => status_view("Starting Bottles", "Loading your workspace.", false),
            Phase::Workspace {
                workspace,
                transition,
                notice,
                ..
            } => match transition {
                WorkspaceTransition::Ready => match notice {
                    Some(error) => notice_view("The experience was not changed", error.to_string()),
                    None => workspace_view(workspace),
                },
                WorkspaceTransition::Confirming(target) => confirmation_view(*target),
                WorkspaceTransition::Draining(_) => status_view(
                    "Preparing to switch experiences",
                    "Finishing current operations safely.",
                    false,
                ),
                WorkspaceTransition::Saving(_) => {
                    status_view("Switching experiences", "Saving your choice.", false)
                }
            },
            Phase::Onboarding {
                state,
                saving,
                notice,
                ..
            } => {
                if saving.is_some() {
                    onboarding_status_view("Finishing setup", "Saving your choice.")
                } else if let Some(error) = notice {
                    onboarding_notice_view("Setup could not be saved", error.to_string())
                } else {
                    state.view().map(AppMessage::Onboarding)
                }
            }
            Phase::ShuttingDown => {
                status_view("Closing Bottles", "Finishing background work.", false)
            }
            Phase::Failed(error) => {
                status_view("Bottles could not start", error.to_string(), false)
            }
        };
        let page: Element<'_, AppMessage> =
            chrome::WindowFrame::new(body, AppMessage::Window).into();
        let dialog = match &self.phase {
            Phase::Workspace {
                workspace: Workspace::Classic(state),
                transition: WorkspaceTransition::Ready,
                notice: None,
                ..
            } => state.dialog().map(|dialog| dialog.map(classic_message)),
            _ => None,
        };

        WindowModal::new(page).dialog(dialog).into()
    }

    fn finish_boot(&mut self, boot: Boot) -> Task<AppMessage> {
        let Boot { config, core } = boot;

        match config.experience {
            None => {
                let state = onboarding::State::new(core.addons().clone());
                self.phase = Phase::Onboarding {
                    core,
                    state: Box::new(state),
                    saving: None,
                    notice: None,
                };
                Task::none()
            }
            Some(experience) => self.open_workspace(core, experience),
        }
    }

    fn request_experience(&mut self, experience: Experience) -> Task<AppMessage> {
        match &mut self.phase {
            Phase::Onboarding { saving, notice, .. } => {
                if saving.is_some() {
                    return Task::none();
                }
                *saving = Some(experience);
                *notice = None;
                save_experience(experience)
            }
            Phase::Workspace {
                workspace,
                transition,
                notice,
                ..
            } => {
                if *transition != WorkspaceTransition::Ready || workspace.experience() == experience
                {
                    return Task::none();
                }

                *transition = WorkspaceTransition::Confirming(experience);
                *notice = None;
                Task::none()
            }
            Phase::Booting | Phase::ShuttingDown | Phase::Failed(_) => Task::none(),
        }
    }

    fn confirm_experience_switch(&mut self) -> Task<AppMessage> {
        let Phase::Workspace {
            workspace,
            transition,
            ..
        } = &mut self.phase
        else {
            return Task::none();
        };
        let WorkspaceTransition::Confirming(target) = *transition else {
            return Task::none();
        };

        workspace.cancel_active_operations();
        *transition = WorkspaceTransition::Draining(target);
        self.advance_experience_switch()
    }

    fn advance_experience_switch(&mut self) -> Task<AppMessage> {
        let target = match &self.phase {
            Phase::Workspace {
                workspace,
                transition: WorkspaceTransition::Draining(target),
                ..
            } if !workspace.has_active_operations() => *target,
            _ => return Task::none(),
        };

        let Phase::Workspace { transition, .. } = &mut self.phase else {
            unreachable!("the switch target came from a workspace")
        };
        *transition = WorkspaceTransition::Saving(target);
        save_experience(target)
    }

    fn finish_experience_save(
        &mut self,
        experience: Experience,
        result: AppResult<()>,
    ) -> Task<AppMessage> {
        let expected = match &self.phase {
            Phase::Onboarding { saving, .. } => *saving == Some(experience),
            Phase::Workspace {
                transition: WorkspaceTransition::Saving(target),
                ..
            } => *target == experience,
            _ => false,
        };
        if !expected {
            return Task::none();
        }

        match result {
            Ok(()) => {
                let core = match &self.phase {
                    Phase::Onboarding { core, .. } | Phase::Workspace { core, .. } => core.clone(),
                    _ => unreachable!("the saved experience belonged to a retained workspace"),
                };
                self.open_workspace(core, experience)
            }
            Err(error) => {
                match &mut self.phase {
                    Phase::Onboarding { saving, notice, .. } => {
                        *saving = None;
                        *notice = Some(error);
                    }
                    Phase::Workspace {
                        workspace,
                        transition,
                        notice,
                        ..
                    } => {
                        *transition = WorkspaceTransition::Ready;
                        *notice = Some(error);
                        return match workspace {
                            Workspace::Classic(state) => {
                                state.resume_after_failed_switch().map(|message| {
                                    AppMessage::Workspace(WorkspaceMessage::Classic(Box::new(
                                        message,
                                    )))
                                })
                            }
                            Workspace::Unavailable(_) => Task::none(),
                        };
                    }
                    _ => unreachable!("the failed save belonged to a retained workspace"),
                }
                Task::none()
            }
        }
    }

    fn open_workspace(&mut self, core: Arc<Bottles>, experience: Experience) -> Task<AppMessage> {
        let (workspace, task) = match experience {
            Experience::Classic => {
                let (state, task) = classic::State::new(core.as_ref());
                (
                    Workspace::Classic(Box::new(state)),
                    task.map(|message| {
                        AppMessage::Workspace(WorkspaceMessage::Classic(Box::new(message)))
                    }),
                )
            }
            Experience::Next => (Workspace::Unavailable(Experience::Next), Task::none()),
        };

        self.phase = Phase::Workspace {
            core,
            workspace,
            transition: WorkspaceTransition::Ready,
            notice: None,
        };
        task
    }

    fn request_close(&mut self) -> Task<AppMessage> {
        let core = match &mut self.phase {
            Phase::Onboarding { core, state, .. } => {
                state.cancel_active_operations();
                core.clone()
            }
            Phase::Workspace {
                core, workspace, ..
            } => {
                workspace.cancel_active_operations();
                core.clone()
            }
            Phase::Booting => {
                self.phase = Phase::ShuttingDown;
                return Task::none();
            }
            Phase::Failed(_) => {
                self.phase = Phase::ShuttingDown;
                return iced::exit();
            }
            Phase::ShuttingDown => return Task::none(),
        };

        self.phase = Phase::ShuttingDown;
        self.shutdown(core)
    }

    fn shutdown(&self, core: Arc<Bottles>) -> Task<AppMessage> {
        Task::perform(
            async move {
                core.shutdown()
                    .await
                    .map_err(|error| AppError::Core(Arc::new(error)))
            },
            AppMessage::ShutdownFinished,
        )
    }
}

fn save_experience(experience: Experience) -> Task<AppMessage> {
    Task::perform(save_config(experience), move |result| {
        AppMessage::ExperienceSaved { experience, result }
    })
}

fn config_path() -> AppResult<PathBuf> {
    directories::ProjectDirs::from("com", "usebottles", "bottles-next")
        .map(|dirs| dirs.config_dir().join(APP_CONFIG_FILE))
        .ok_or(AppError::ConfigDirectory)
}

async fn load_config() -> AppResult<AppConfig> {
    let path = config_path()?;

    load_config_from(&path).await
}

async fn load_config_from(path: &Path) -> AppResult<AppConfig> {
    match next_config::load(&path).await {
        Ok(config) => Ok(config),
        Err(next_config::error::Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            Ok(AppConfig::default())
        }
        Err(source) => Err(AppError::Config {
            action: "load",
            path: path.to_owned(),
            source: Arc::new(source),
        }),
    }
}

async fn save_config(experience: Experience) -> AppResult<()> {
    let path = config_path()?;
    save_config_to(&path, experience).await
}

async fn save_config_to(path: &Path, experience: Experience) -> AppResult<()> {
    next_config::save(
        path,
        &AppConfig {
            experience: Some(experience),
        },
    )
    .await
    .map_err(|source| AppError::Config {
        action: "save",
        path: path.to_owned(),
        source: Arc::new(source),
    })
}

async fn boot() -> AppResult<Boot> {
    let config = load_config().await?;
    let core = Bottles::open(core_config())
        .await
        .map(Arc::new)
        .map_err(|error| AppError::Core(Arc::new(error)))?;

    Ok(Boot { config, core })
}

fn core_config() -> CoreConfig {
    CoreConfig {
        component_catalog: Some(
            COMPONENT_CATALOG_URL
                .parse()
                .expect("component catalog URL is valid"),
        ),
        dependency_catalog: Some(
            DEPENDENCY_CATALOG_URL
                .parse()
                .expect("dependency catalog URL is valid"),
        ),
        ..CoreConfig::default()
    }
}

fn status_view<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
    offer_classic: bool,
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    let mut status = column![text(title).size(32), text(description)].spacing(12);
    if offer_classic {
        status = status.push(
            Button::new("Use Classic")
                .kind(ButtonKind::Primary)
                .on_press(AppMessage::RequestExperience(Experience::Classic)),
        );
    }

    root_body(status)
}

fn onboarding_status_view<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    onboarding::shell(
        column![text(title).size(32), text(description)].spacing(12),
        AppMessage::Window(chrome::Action::Drag),
    )
}

fn onboarding_notice_view<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    onboarding::shell(
        column![
            text(title).size(32),
            text(description),
            Button::new("Continue")
                .kind(ButtonKind::Primary)
                .on_press(AppMessage::DismissNotice),
        ]
        .spacing(12),
        AppMessage::Window(chrome::Action::Drag),
    )
}

fn classic_message(message: classic::Message) -> AppMessage {
    AppMessage::Workspace(WorkspaceMessage::Classic(Box::new(message)))
}

fn workspace_view(workspace: &Workspace) -> Element<'_, AppMessage> {
    match workspace {
        Workspace::Classic(state) => state.view().map(classic_message),
        Workspace::Unavailable(Experience::Next) => status_view(
            "Next experience is not available yet",
            "Choose Classic to use Bottles today.",
            true,
        ),
        Workspace::Unavailable(Experience::Classic) => {
            unreachable!("the Classic experience always has a workspace")
        }
    }
}

fn confirmation_view(target: Experience) -> Element<'static, AppMessage> {
    use iced::widget::{column, row, text};

    let (description, confirm_label) = match target {
        Experience::Classic => (
            "Current operations will finish before Classic opens.",
            "Switch to Classic",
        ),
        Experience::Next => (
            "Current operations will finish before Next opens.",
            "Switch to Next",
        ),
    };
    let actions = row![
        Button::new("Cancel")
            .kind(ButtonKind::Transparent)
            .on_press(AppMessage::CancelExperienceSwitch),
        Button::new(confirm_label)
            .kind(ButtonKind::Primary)
            .on_press(AppMessage::ConfirmExperienceSwitch),
    ]
    .spacing(8);

    root_body(
        column![
            text("Switch experiences?").size(32),
            text(description),
            actions
        ]
        .spacing(12),
    )
}

fn notice_view<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    root_body(
        column![
            text(title).size(32),
            text(description),
            Button::new("Continue")
                .kind(ButtonKind::Primary)
                .on_press(AppMessage::DismissNotice),
        ]
        .spacing(12),
    )
}

fn root_body<'a>(content: impl Into<Element<'a, AppMessage>>) -> Element<'a, AppMessage> {
    use iced::widget::{center, column};

    let content = column![
        HeaderBar::new(AppMessage::Window(chrome::Action::Drag)),
        center(content).width(Fill).height(Fill),
    ]
    .width(Fill)
    .height(Fill);

    content.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_catalogs_are_configured() {
        let config = core_config();

        assert_eq!(
            config.component_catalog.as_ref().map(|url| url.as_str()),
            Some(COMPONENT_CATALOG_URL)
        );
        assert_eq!(
            config.dependency_catalog.as_ref().map(|url| url.as_str()),
            Some(DEPENDENCY_CATALOG_URL)
        );
    }

    #[test]
    fn a_missing_config_has_no_selected_experience() {
        let path = std::env::temp_dir().join(format!("next-ui-{}.toml", uuid::Uuid::new_v4()));
        let config = futures_lite::future::block_on(load_config_from(&path)).unwrap();

        assert_eq!(config.experience, None);
    }

    #[test]
    fn a_saved_experience_round_trips() {
        let path = std::env::temp_dir().join(format!("next-ui-{}.toml", uuid::Uuid::new_v4()));

        futures_lite::future::block_on(async {
            save_config_to(&path, Experience::Classic).await.unwrap();
            let config = load_config_from(&path).await.unwrap();
            assert_eq!(config.experience, Some(Experience::Classic));
        });

        std::fs::remove_file(path).unwrap();
    }
}
