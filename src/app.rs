use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use bottles_core::{
    Bottle, BottleManager, Bottles, Config as CoreConfig, Profiles, ProfilesConfig,
    error::Error as CoreError,
};
use iced::{Element, Fill, Subscription, Task, theme::Mode as ThemeMode, window};
use next_config::Config;
use serde::{Deserialize, Serialize};

use crate::{
    chrome, classic,
    domain::DomainSnapshots,
    features, next, onboarding, theme,
    widget::{Button, ButtonKind, Dialog, WindowModal},
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Config)]
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
    theme: iced::Theme,
    next_run_generation: u64,
}

enum Phase {
    Booting { close_requested: bool },
    ClosingBoot(Boot),
    Session(Session),
    Failed(AppError),
}

struct Session {
    config: AppConfig,
    core: Arc<Bottles>,
    domain: DomainSnapshots,
    screen: SessionScreen,
    lifecycle: SessionLifecycle,
    overlay: Option<GlobalOverlay>,
}

enum SessionScreen {
    Onboarding(Box<onboarding::State>),
    Running {
        generation: u64,
        features: Box<features::State>,
        presentation: Presentation,
    },
}

enum GlobalOverlay {
    ConfirmExperienceSwitch(Experience),
    Notice(AppError),
}

enum SessionLifecycle {
    Active,
    Draining(DrainIntent),
    Saving {
        candidate: AppConfig,
        close_requested: bool,
    },
    ShuttingDown(ShutdownResume),
}

impl Session {
    fn has_active_operations(&self) -> bool {
        match &self.screen {
            SessionScreen::Onboarding(state) => state.has_active_operations(),
            SessionScreen::Running { features, .. } => features.has_active_operations(),
        }
    }

    fn cancel_active_operations(&mut self) {
        match &mut self.screen {
            SessionScreen::Onboarding(state) => state.cancel_active_operations(),
            SessionScreen::Running { features, .. } => features.cancel_active_operations(),
        }
    }
}

enum Presentation {
    Classic(Box<classic::State>),
    Next(next::State),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrainIntent {
    Switch(Experience),
    Close,
}

enum ShutdownResume {
    Existing,
    Reopen(AppConfig),
}

#[derive(Clone)]
pub(crate) enum AppMessage {
    Action(AppAction),
    Event(AppEvent),
}

#[derive(Clone)]
pub(crate) enum AppAction {
    Onboarding(onboarding::Action),
    Classic(Box<classic::Message>),
    Feature(features::Message),
    Next(next::Action),
    ConfirmExperienceSwitch,
    CancelExperienceSwitch,
    DismissNotice,
    Window(chrome::Action),
}

#[derive(Clone)]
pub(crate) enum AppEvent {
    Booted(AppResult<Boot>),
    Onboarding(onboarding::Event),
    Feature {
        generation: u64,
        message: features::Message,
    },
    BottlesChanged(Vec<Bottle>),
    ProfilesChanged(Arc<ProfilesConfig>),
    ConfigSaved(AppResult<()>),
    ShutdownFinished(AppResult<()>),
    SystemThemeChanged(ThemeMode),
}

impl fmt::Debug for AppMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Action(_) => "Action",
            Self::Event(_) => "Event",
        })
    }
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
                phase: Phase::Booting {
                    close_requested: false,
                },
                theme: theme::for_mode(ThemeMode::default()),
                next_run_generation: 0,
            },
            Task::batch([
                Task::perform(boot(), |result| AppMessage::Event(AppEvent::Booted(result))),
                iced::system::theme()
                    .map(|mode| AppMessage::Event(AppEvent::SystemThemeChanged(mode))),
            ]),
        )
    }

    pub(crate) fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }

    pub(crate) fn subscription(&self) -> Subscription<AppMessage> {
        let mut subscriptions = vec![
            iced::system::theme_changes()
                .map(|mode| AppMessage::Event(AppEvent::SystemThemeChanged(mode))),
            chrome::subscription().map(|action| AppMessage::Action(AppAction::Window(action))),
        ];

        if let Phase::Session(session) = &self.phase {
            subscriptions.extend(domain_subscriptions(session));
        }

        Subscription::batch(subscriptions)
    }

    pub(crate) fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Action(action) => self.update_action(action),
            AppMessage::Event(event) => self.update_event(event),
        }
    }

    fn update_action(&mut self, action: AppAction) -> Task<AppMessage> {
        if matches!(action, AppAction::Window(chrome::Action::RequestClose)) {
            return self.request_close();
        }

        let Phase::Session(session) = &self.phase else {
            return Task::none();
        };
        if !matches!(session.lifecycle, SessionLifecycle::Active) {
            return Task::none();
        }

        if let AppAction::Window(action) = action {
            return match action {
                chrome::Action::Drag => window::latest().and_then(window::drag),
                chrome::Action::Resize(direction) => {
                    window::latest().and_then(move |id| window::drag_resize(id, direction))
                }
                chrome::Action::FocusPrevious => iced::widget::operation::focus_previous(),
                chrome::Action::FocusNext => iced::widget::operation::focus_next(),
                chrome::Action::RequestClose => {
                    unreachable!("close requests are handled before the action gate")
                }
            };
        }

        match action {
            AppAction::Onboarding(onboarding::Action::Finished(experience)) => {
                self.begin_onboarding_switch(experience)
            }
            AppAction::Onboarding(action) => {
                let Phase::Session(session) = &mut self.phase else {
                    return Task::none();
                };
                let SessionScreen::Onboarding(state) = &mut session.screen else {
                    return Task::none();
                };
                state.update_action(action).map(onboarding_event_message)
            }
            AppAction::Classic(action) => {
                let Phase::Session(session) = &mut self.phase else {
                    return Task::none();
                };
                let SessionScreen::Running {
                    generation,
                    features,
                    presentation: Presentation::Classic(state),
                    ..
                } = &mut session.screen
                else {
                    return Task::none();
                };
                let generation = *generation;

                state
                    .update(features, *action, &session.domain)
                    .map(move |message| feature_event_message(generation, message))
            }
            AppAction::Feature(message) => self.update_feature(message),
            AppAction::Next(action) => {
                let Phase::Session(session) = &self.phase else {
                    return Task::none();
                };
                if !matches!(
                    session.screen,
                    SessionScreen::Running {
                        presentation: Presentation::Next(_),
                        ..
                    }
                ) {
                    return Task::none();
                }

                match action {
                    next::Action::UseClassic => self.request_experience(Experience::Classic),
                }
            }
            AppAction::ConfirmExperienceSwitch => self.confirm_experience_switch(),
            AppAction::CancelExperienceSwitch => {
                if let Phase::Session(session) = &mut self.phase {
                    session.overlay = None;
                }
                Task::none()
            }
            AppAction::DismissNotice => {
                if let Phase::Session(session) = &mut self.phase {
                    session.overlay = None;
                }
                Task::none()
            }
            AppAction::Window(_) => unreachable!(),
        }
    }

    fn update_event(&mut self, event: AppEvent) -> Task<AppMessage> {
        match event {
            AppEvent::Booted(result) => self.finish_boot(result),
            AppEvent::Onboarding(event) => {
                let Phase::Session(session) = &mut self.phase else {
                    return Task::none();
                };
                if !matches!(
                    session.lifecycle,
                    SessionLifecycle::Active | SessionLifecycle::Draining(_)
                ) {
                    return Task::none();
                }
                let SessionScreen::Onboarding(state) = &mut session.screen else {
                    return Task::none();
                };
                let task = state.update_event(event).map(onboarding_event_message);
                Task::batch([task, self.advance_draining()])
            }
            AppEvent::Feature {
                generation,
                message,
            } => {
                let Phase::Session(session) = &mut self.phase else {
                    return Task::none();
                };
                if !matches!(
                    session.lifecycle,
                    SessionLifecycle::Active | SessionLifecycle::Draining(_)
                ) {
                    return Task::none();
                }
                let SessionScreen::Running {
                    generation: current_generation,
                    ..
                } = &mut session.screen
                else {
                    return Task::none();
                };
                if *current_generation != generation {
                    return Task::none();
                }
                let task = self.update_feature(message);
                Task::batch([task, self.advance_draining()])
            }
            AppEvent::BottlesChanged(bottles) => self.update_bottles(bottles),
            AppEvent::ProfilesChanged(profiles) => self.update_profiles(profiles),
            AppEvent::ConfigSaved(result) => self.finish_config_save(result),
            AppEvent::ShutdownFinished(result) => self.finish_shutdown(result),
            AppEvent::SystemThemeChanged(mode) => {
                self.theme = theme::for_mode(mode);
                Task::none()
            }
        }
    }

    fn update_feature(&mut self, message: features::Message) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        let SessionScreen::Running {
            generation,
            features,
            presentation,
        } = &mut session.screen
        else {
            return Task::none();
        };
        let generation = *generation;
        let (task, output) = features.update(message, &session.domain);

        if let (Presentation::Classic(state), Some(output)) = (presentation, output) {
            state.handle_feature_output(output);
        }

        task.map(move |message| feature_event_message(generation, message))
    }

    fn update_bottles(&mut self, bottles: Vec<Bottle>) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !session.domain.replace_bottles(bottles) {
            return Task::none();
        }
        let effects_allowed = matches!(session.lifecycle, SessionLifecycle::Active);
        let SessionScreen::Running {
            generation,
            features,
            presentation,
        } = &mut session.screen
        else {
            return Task::none();
        };
        let generation = *generation;
        if let Presentation::Classic(state) = presentation {
            state.bottles_changed(features, &session.domain);
        }
        if effects_allowed {
            features
                .reload_library()
                .map(move |message| feature_event_message(generation, message))
        } else {
            Task::none()
        }
    }

    fn update_profiles(&mut self, profiles: Arc<ProfilesConfig>) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        let Some(selected_changed) = session.domain.replace_profiles(profiles) else {
            return Task::none();
        };
        let effects_allowed = matches!(session.lifecycle, SessionLifecycle::Active);
        let SessionScreen::Running {
            generation,
            features,
            ..
        } = &mut session.screen
        else {
            return Task::none();
        };
        let generation = *generation;
        if effects_allowed {
            features
                .profiles_changed(&session.domain, selected_changed)
                .map(move |message| feature_event_message(generation, message))
        } else {
            Task::none()
        }
    }

    pub(crate) fn view(&self) -> Element<'_, AppMessage> {
        let body = match &self.phase {
            Phase::Booting {
                close_requested: false,
                ..
            } => status_view("Starting Bottles", "Loading your workspace."),
            Phase::Booting {
                close_requested: true,
                ..
            }
            | Phase::ClosingBoot(_) => status_view("Closing Bottles", "Finishing background work."),
            Phase::Session(session) => match &session.lifecycle {
                SessionLifecycle::Active => match &session.screen {
                    SessionScreen::Onboarding(state) => {
                        if let Some(GlobalOverlay::Notice(error)) = &session.overlay {
                            onboarding_notice_view("Setup could not be saved", error.to_string())
                        } else {
                            state.view().map(onboarding_action_message)
                        }
                    }
                    SessionScreen::Running { .. } => presentation_view(session),
                },
                SessionLifecycle::Draining(intent) => match intent {
                    DrainIntent::Switch(_) => status_view(
                        "Preparing to switch experiences",
                        "Finishing current operations safely.",
                    ),
                    DrainIntent::Close => {
                        status_view("Closing Bottles", "Finishing background work.")
                    }
                },
                SessionLifecycle::Saving {
                    close_requested: true,
                    ..
                }
                | SessionLifecycle::ShuttingDown(_) => {
                    status_view("Closing Bottles", "Finishing background work.")
                }
                SessionLifecycle::Saving {
                    close_requested: false,
                    ..
                } => match session.screen {
                    SessionScreen::Onboarding(_) => {
                        onboarding_status_view("Finishing setup", "Saving your choice.")
                    }
                    SessionScreen::Running { .. } => {
                        status_view("Switching experiences", "Saving your choice.")
                    }
                },
            },
            Phase::Failed(error) => status_view("Bottles could not start", error.to_string()),
        };
        let page =
            chrome::window_frame(body, |action| AppMessage::Action(AppAction::Window(action)));
        let dialog = match &self.phase {
            Phase::Session(Session {
                screen:
                    SessionScreen::Running {
                        features,
                        presentation,
                        ..
                    },
                lifecycle: SessionLifecycle::Active,
                overlay,
                ..
            }) => match overlay {
                Some(GlobalOverlay::ConfirmExperienceSwitch(target)) => {
                    Some(confirmation_dialog(*target))
                }
                Some(GlobalOverlay::Notice(error)) => Some(notice_dialog(
                    "The experience was not changed",
                    error.to_string(),
                )),
                None => match presentation {
                    Presentation::Classic(state) => state.dialog(features).map(|dialog| {
                        dialog.map(|message| {
                            AppMessage::Action(AppAction::Classic(Box::new(message)))
                        })
                    }),
                    Presentation::Next(_) => None,
                },
            },
            _ => None,
        };

        WindowModal::new(page).dialog(dialog).into()
    }

    fn finish_boot(&mut self, result: AppResult<Boot>) -> Task<AppMessage> {
        let Phase::Booting { close_requested } = &self.phase else {
            return Task::none();
        };
        let close_requested = *close_requested;

        match result {
            Ok(boot) if close_requested => {
                let core = boot.core.clone();
                self.phase = Phase::ClosingBoot(boot);
                shutdown_task(core)
            }
            Ok(boot) => self.open_boot(boot, None),
            Err(_) if close_requested => iced::exit(),
            Err(error) => {
                self.phase = Phase::Failed(error);
                Task::none()
            }
        }
    }

    fn begin_onboarding_switch(&mut self, experience: Experience) -> Task<AppMessage> {
        let Phase::Session(session) = &self.phase else {
            return Task::none();
        };
        if !matches!(session.screen, SessionScreen::Onboarding(_)) {
            return Task::none();
        }
        self.begin_draining(DrainIntent::Switch(experience))
    }

    fn request_experience(&mut self, experience: Experience) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !matches!(session.screen, SessionScreen::Running { .. }) {
            return Task::none();
        }
        if session.config.experience == Some(experience) || session.overlay.is_some() {
            return Task::none();
        }

        session.overlay = Some(GlobalOverlay::ConfirmExperienceSwitch(experience));
        Task::none()
    }

    fn confirm_experience_switch(&mut self) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !matches!(session.screen, SessionScreen::Running { .. }) {
            return Task::none();
        }
        let target = match session.overlay {
            Some(GlobalOverlay::ConfirmExperienceSwitch(target)) => target,
            _ => return Task::none(),
        };
        session.overlay = None;

        self.begin_draining(DrainIntent::Switch(target))
    }

    fn begin_draining(&mut self, intent: DrainIntent) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !matches!(session.lifecycle, SessionLifecycle::Active) {
            return Task::none();
        }
        session.cancel_active_operations();
        session.lifecycle = SessionLifecycle::Draining(intent);
        self.advance_draining()
    }

    fn advance_draining(&mut self) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        let SessionLifecycle::Draining(intent) = session.lifecycle else {
            return Task::none();
        };
        if session.has_active_operations() {
            return Task::none();
        }

        match intent {
            DrainIntent::Switch(experience) => {
                let mut candidate = session.config.clone();
                candidate.experience = Some(experience);
                session.lifecycle = SessionLifecycle::Saving {
                    candidate: candidate.clone(),
                    close_requested: false,
                };
                save_config_task(candidate)
            }
            DrainIntent::Close => {
                let core = session.core.clone();
                session.lifecycle = SessionLifecycle::ShuttingDown(ShutdownResume::Existing);
                shutdown_task(core)
            }
        }
    }

    fn finish_config_save(&mut self, result: AppResult<()>) -> Task<AppMessage> {
        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !matches!(session.lifecycle, SessionLifecycle::Saving { .. }) {
            return Task::none();
        }
        let SessionLifecycle::Saving {
            candidate,
            close_requested,
        } = std::mem::replace(&mut session.lifecycle, SessionLifecycle::Active)
        else {
            unreachable!()
        };

        match result {
            Ok(()) if close_requested => {
                let core = session.core.clone();
                session.lifecycle =
                    SessionLifecycle::ShuttingDown(ShutdownResume::Reopen(candidate));
                shutdown_task(core)
            }
            Ok(()) => {
                let core = session.core.clone();
                self.open_experience(core, candidate, None)
            }
            Err(_) if close_requested => {
                let core = session.core.clone();
                session.lifecycle = SessionLifecycle::ShuttingDown(ShutdownResume::Existing);
                shutdown_task(core)
            }
            Err(error) => {
                session.overlay = Some(GlobalOverlay::Notice(error));
                resume_session(session)
            }
        }
    }

    fn request_close(&mut self) -> Task<AppMessage> {
        match &mut self.phase {
            Phase::Booting { close_requested } => {
                *close_requested = true;
                return Task::none();
            }
            Phase::ClosingBoot(_) => return Task::none(),
            Phase::Failed(_) => return iced::exit(),
            Phase::Session(session) => match &mut session.lifecycle {
                SessionLifecycle::Active => {}
                SessionLifecycle::Draining(intent) => {
                    *intent = DrainIntent::Close;
                    return self.advance_draining();
                }
                SessionLifecycle::Saving {
                    close_requested, ..
                } => {
                    *close_requested = true;
                    return Task::none();
                }
                SessionLifecycle::ShuttingDown(_) => return Task::none(),
            },
        }

        self.begin_draining(DrainIntent::Close)
    }

    fn finish_shutdown(&mut self, result: AppResult<()>) -> Task<AppMessage> {
        if let Phase::ClosingBoot(boot) = &self.phase {
            let boot = boot.clone();
            return match result {
                Ok(()) => iced::exit(),
                Err(error) => self.open_boot(boot, Some(error)),
            };
        }

        let Phase::Session(session) = &mut self.phase else {
            return Task::none();
        };
        if !matches!(session.lifecycle, SessionLifecycle::ShuttingDown(_)) {
            return Task::none();
        }
        let SessionLifecycle::ShuttingDown(resume) =
            std::mem::replace(&mut session.lifecycle, SessionLifecycle::Active)
        else {
            unreachable!()
        };

        match (result, resume) {
            (Ok(()), _) => iced::exit(),
            (Err(error), ShutdownResume::Existing) => {
                session.overlay = Some(GlobalOverlay::Notice(error));
                resume_session(session)
            }
            (Err(error), ShutdownResume::Reopen(config)) => {
                let core = session.core.clone();
                self.open_experience(core, config, Some(error))
            }
        }
    }

    fn open_boot(&mut self, boot: Boot, notice: Option<AppError>) -> Task<AppMessage> {
        let Boot { config, core } = boot;
        match config.experience {
            None => {
                let domain = DomainSnapshots::new(&core);
                self.phase = Phase::Session(Session {
                    config,
                    domain,
                    screen: SessionScreen::Onboarding(Box::new(onboarding::State::new(
                        core.addons().clone(),
                    ))),
                    core,
                    lifecycle: SessionLifecycle::Active,
                    overlay: notice.map(GlobalOverlay::Notice),
                });
                Task::none()
            }
            Some(_) => self.open_experience(core, config, notice),
        }
    }

    fn open_experience(
        &mut self,
        core: Arc<Bottles>,
        config: AppConfig,
        notice: Option<AppError>,
    ) -> Task<AppMessage> {
        let Some(experience) = config.experience else {
            unreachable!("a presentation requires a selected experience")
        };
        let generation = self.next_run_generation;
        self.next_run_generation = self.next_run_generation.wrapping_add(1);
        let domain = match &self.phase {
            Phase::Session(session) if Arc::ptr_eq(&session.core, &core) => session.domain.clone(),
            _ => DomainSnapshots::new(&core),
        };
        let features = features::State::new(core.as_ref(), &domain);
        let presentation = match experience {
            Experience::Classic => Presentation::Classic(Box::new(classic::State::new())),
            Experience::Next => Presentation::Next(next::State::new()),
        };

        self.phase = Phase::Session(Session {
            config,
            core,
            domain,
            screen: SessionScreen::Running {
                generation,
                features: Box::new(features),
                presentation,
            },
            lifecycle: SessionLifecycle::Active,
            overlay: notice.map(GlobalOverlay::Notice),
        });
        Task::none()
    }
}

fn resume_session(session: &mut Session) -> Task<AppMessage> {
    let SessionScreen::Running {
        generation,
        features,
        presentation,
        ..
    } = &mut session.screen
    else {
        return Task::none();
    };
    let generation = *generation;
    features
        .profiles
        .sync_selected(session.domain.profiles().selected());

    match presentation {
        Presentation::Classic(state) => state
            .resume_after_failed_switch(features, &session.domain)
            .map(move |message| feature_event_message(generation, message)),
        Presentation::Next(_) => Task::none(),
    }
}

fn shutdown_task(core: Arc<Bottles>) -> Task<AppMessage> {
    Task::perform(
        async move {
            core.shutdown()
                .await
                .map_err(|error| AppError::Core(Arc::new(error)))
        },
        |result| AppMessage::Event(AppEvent::ShutdownFinished(result)),
    )
}

fn domain_subscriptions(session: &Session) -> [Subscription<AppMessage>; 2] {
    [
        Subscription::run_with(session.core.bottles().clone(), BottleManager::watch)
            .map(|bottles| AppMessage::Event(AppEvent::BottlesChanged(bottles))),
        Subscription::run_with(session.core.profiles().clone(), Profiles::watch)
            .map(|profiles| AppMessage::Event(AppEvent::ProfilesChanged(profiles))),
    ]
}

fn onboarding_event_message(event: onboarding::Event) -> AppMessage {
    AppMessage::Event(AppEvent::Onboarding(event))
}

fn onboarding_action_message(action: onboarding::Action) -> AppMessage {
    match action {
        onboarding::Action::DragWindow => {
            AppMessage::Action(AppAction::Window(chrome::Action::Drag))
        }
        action => AppMessage::Action(AppAction::Onboarding(action)),
    }
}

fn classic_action_message(message: classic::Message) -> AppMessage {
    match message {
        classic::Message::DragWindow => AppMessage::Action(AppAction::Window(chrome::Action::Drag)),
        classic::Message::Feature(message) => AppMessage::Action(AppAction::Feature(message)),
        message => AppMessage::Action(AppAction::Classic(Box::new(message))),
    }
}

fn feature_event_message(generation: u64, message: features::Message) -> AppMessage {
    AppMessage::Event(AppEvent::Feature {
        generation,
        message,
    })
}

fn save_config_task(config: AppConfig) -> Task<AppMessage> {
    Task::perform(save_config(config), |result| {
        AppMessage::Event(AppEvent::ConfigSaved(result))
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
    match next_config::load(path).await {
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

async fn save_config(config: AppConfig) -> AppResult<()> {
    let path = config_path()?;
    save_config_to(&path, &config).await
}

async fn save_config_to(path: &Path, config: &AppConfig) -> AppResult<()> {
    next_config::save(path, config)
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
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    root_body(column![text(title).size(32), text(description)].spacing(12))
}

fn onboarding_status_view<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, AppMessage> {
    use iced::widget::{column, text};

    onboarding::shell(
        column![text(title).size(32), text(description)].spacing(12),
        AppMessage::Action(AppAction::Window(chrome::Action::Drag)),
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
                .on_press(AppMessage::Action(AppAction::DismissNotice)),
        ]
        .spacing(12),
        AppMessage::Action(AppAction::Window(chrome::Action::Drag)),
    )
}

fn presentation_view(session: &Session) -> Element<'_, AppMessage> {
    let SessionScreen::Running {
        features,
        presentation,
        ..
    } = &session.screen
    else {
        unreachable!("only running sessions render a presentation")
    };
    match presentation {
        Presentation::Classic(state) => state
            .view(features, &session.domain)
            .map(classic_action_message),
        Presentation::Next(state) => root_body(
            state
                .view()
                .map(|action| AppMessage::Action(AppAction::Next(action))),
        ),
    }
}

fn confirmation_dialog(target: Experience) -> Dialog<'static, AppMessage> {
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
            .on_press(AppMessage::Action(AppAction::CancelExperienceSwitch)),
        Button::new(confirm_label)
            .kind(ButtonKind::Primary)
            .on_press(AppMessage::Action(AppAction::ConfirmExperienceSwitch)),
    ]
    .spacing(8);

    Dialog::new(
        column![
            text("Switch experiences?").size(32),
            text(description),
            actions
        ]
        .spacing(12),
        AppMessage::Action(AppAction::CancelExperienceSwitch),
    )
}

fn notice_dialog<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    description: impl iced::widget::text::IntoFragment<'a>,
) -> Dialog<'a, AppMessage> {
    use iced::widget::{column, text};

    Dialog::new(
        column![
            text(title).size(32),
            text(description),
            Button::new("Continue")
                .kind(ButtonKind::Primary)
                .on_press(AppMessage::Action(AppAction::DismissNotice)),
        ]
        .spacing(12),
        AppMessage::Action(AppAction::DismissNotice),
    )
}

fn root_body<'a>(content: impl Into<Element<'a, AppMessage>>) -> Element<'a, AppMessage> {
    use iced::widget::{center, column};

    column![
        chrome::header(
            AppMessage::Action(AppAction::Window(chrome::Action::Drag)),
            true,
            |header| header,
        ),
        center(content).width(Fill).height(Fill),
    ]
    .width(Fill)
    .height(Fill)
    .into()
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

        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn a_saved_experience_round_trips() {
        let path = std::env::temp_dir().join(format!("next-ui-{}.toml", uuid::Uuid::new_v4()));
        let expected = AppConfig {
            experience: Some(Experience::Classic),
        };

        futures_lite::future::block_on(async {
            save_config_to(&path, &expected).await.unwrap();
            assert_eq!(load_config_from(&path).await.unwrap(), expected);
        });

        std::fs::remove_file(path).unwrap();
    }
}
