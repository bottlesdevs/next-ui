//! The application shell: lifecycle, window, overlays, and the shown screen.

use std::fmt;

use iced::{Element, Fill, Subscription, Task, theme::Mode as ThemeMode, window};
use next_ui::{
    chrome,
    layout::{PaneContext, Side, side_panel},
    theme,
    widget::text_size,
};
use uuid::Uuid;

use crate::{
    add_program, classic, manage_profiles,
    model::{self, Action, Boot, Experience, Model},
    next,
    notifications::{self, Follow, Notifications},
    onboarding, preferences,
};

pub(crate) struct App {
    theme: iced::Theme,
    state: State,
    closing: bool,
}

enum State {
    Loading,
    Failed(String),
    Ready(Box<Ready>),
}

/// The model, the shown screen, and the app-wide surfaces over it.
struct Ready {
    model: Model,
    notifications: Notifications,
    screen: Screen,
    /// Preferences, shown instead of the screen while open.
    preferences: Option<preferences::State>,
    /// The Manage Profiles pane at the end side.
    profiles: manage_profiles::State,
    /// The Add Program pane at the start side, kept while it slides closed.
    adding: Option<add_program::State>,
    adding_open: bool,
}

enum Screen {
    Onboarding(onboarding::State),
    Classic(classic::State),
    Next(next::State),
}

impl Screen {
    fn new(experience: Experience) -> Self {
        match experience {
            Experience::Next if experience.is_available() => Self::Next(next::State::new()),
            Experience::Next | Experience::Classic => Self::Classic(classic::State::new()),
        }
    }
}

/// What a screen asks of the shell.
pub(crate) enum Request {
    Act(Action),
    OpenPreferences,
    ClosePreferences,
    /// Opens the Add Program pane for programs going to `target`.
    AddProgram(add_program::Target),
    /// Asks for a program, then runs it once in bottle `prefix`.
    RunProgram(Uuid),
    /// Passes on what an app-wide header control sent.
    Control(Control),
}

/// What a program chosen in the file picker is for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Pick {
    /// The Add Program pane's file.
    AddProgram,
    /// A one-off run in the bottle with this prefix.
    Run(Uuid),
}

/// The extensions of the files Wine runs as Windows programs.
const WINDOWS_PROGRAMS: [&str; 3] = ["exe", "msi", "bat"];

/// What the app-wide header controls send. Screens place the controls, as
/// only they build their headers, and pass this on unread.
#[derive(Debug, Clone)]
pub(crate) enum Control {
    Profiles(manage_profiles::Message),
    CancelDownload(Uuid),
}

#[derive(Clone)]
pub(crate) enum Message {
    Booted(Result<Boot, String>),
    ThemeChanged(ThemeMode),
    Window(chrome::Action),
    Model(model::Message),
    DismissNotification(Uuid),
    CopyNotification(Uuid),
    /// Runs a notification's action and dismisses it.
    FollowNotification(Uuid),
    /// The program chosen in the file picker, or `None` when cancelled.
    Picked(Pick, Option<std::path::PathBuf>),
    ExpandNotifications,
    DismissAllNotifications,
    ShutdownFinished(Result<(), String>),
    Onboarding(onboarding::Message),
    Classic(classic::Message),
    Next(next::Message),
    Preferences(preferences::Message),
    Profiles(manage_profiles::Message),
    Adding(add_program::Message),
}

impl fmt::Debug for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Booted(_) => "Booted",
            Self::ThemeChanged(_) => "ThemeChanged",
            Self::Window(_) => "Window",
            Self::Model(_) => "Model",
            Self::DismissNotification(_) => "DismissNotification",
            Self::CopyNotification(_) => "CopyNotification",
            Self::FollowNotification(_) => "FollowNotification",
            Self::Picked(..) => "Picked",
            Self::ExpandNotifications => "ExpandNotifications",
            Self::DismissAllNotifications => "DismissAllNotifications",
            Self::ShutdownFinished(_) => "ShutdownFinished",
            Self::Onboarding(_) => "Onboarding",
            Self::Classic(_) => "Classic",
            Self::Next(_) => "Next",
            Self::Preferences(_) => "Preferences",
            Self::Profiles(_) => "Profiles",
            Self::Adding(_) => "Adding",
        })
    }
}

impl App {
    pub(crate) fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme: theme::for_mode(ThemeMode::default()),
                state: State::Loading,
                closing: false,
            },
            Task::batch([
                Task::perform(Model::boot(), Message::Booted),
                iced::system::theme().map(Message::ThemeChanged),
            ]),
        )
    }

    pub(crate) fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let model = match &self.state {
            State::Ready(ready) => ready.model.subscription().map(Message::Model),
            State::Loading | State::Failed(_) => Subscription::none(),
        };
        Subscription::batch([
            chrome::subscription().map(Message::Window),
            iced::system::theme_changes().map(Message::ThemeChanged),
            model,
        ])
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeChanged(mode) => self.theme = theme::for_mode(mode),
            Message::Window(chrome::Action::RequestClose) => {
                if self.closing {
                    return Task::none();
                }
                self.closing = true;
                match &self.state {
                    State::Loading => {}
                    State::Failed(_) => return iced::exit(),
                    State::Ready(ready) => return shutdown(&ready.model),
                }
            }
            Message::Window(action) => {
                return match action {
                    chrome::Action::Drag => window::latest().and_then(window::drag),
                    chrome::Action::Resize(direction) => {
                        window::latest().and_then(move |id| window::drag_resize(id, direction))
                    }
                    chrome::Action::FocusPrevious => iced::widget::operation::focus_previous(),
                    chrome::Action::FocusNext => iced::widget::operation::focus_next(),
                    chrome::Action::RequestClose => unreachable!(),
                };
            }
            Message::Booted(result) => match result {
                Ok(boot) => {
                    let (model, watch) = Model::new(boot);
                    let screen = model
                        .settings
                        .experience
                        .map(Screen::new)
                        .unwrap_or_else(|| Screen::Onboarding(onboarding::State::new()));
                    let close = if self.closing {
                        shutdown(&model)
                    } else {
                        Task::none()
                    };
                    let mut ready = Ready {
                        model,
                        notifications: Notifications::default(),
                        screen,
                        preferences: None,
                        profiles: manage_profiles::State::default(),
                        adding: None,
                        adding_open: false,
                    };
                    ready.notify();
                    self.state = State::Ready(Box::new(ready));
                    return Task::batch([watch.map(Message::Model), close]);
                }
                Err(error) => {
                    self.state = State::Failed(error);
                    if self.closing {
                        return iced::exit();
                    }
                }
            },
            Message::ShutdownFinished(result) => {
                if let Err(error) = result {
                    tracing::error!(%error, "core shutdown failed");
                }
                return iced::exit();
            }
            message => {
                let State::Ready(ready) = &mut self.state else {
                    return Task::none();
                };
                return ready.update(message, self.closing);
            }
        }
        Task::none()
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        use iced::widget::stack;
        let body = if self.closing {
            status("Closing Bottles", "Finishing operations…")
        } else {
            match &self.state {
                State::Loading => status("Bottles Next", "Loading…"),
                State::Failed(error) => status("Could not start Bottles", error),
                State::Ready(ready) => {
                    let mut layers = stack![ready.view()].width(Fill).height(Fill);
                    if !ready.notifications.is_empty() {
                        layers = layers.push(notifications::layer(&ready.notifications));
                    }
                    layers.into()
                }
            }
        };
        chrome::window_frame(body, Message::Window)
    }
}

impl Ready {
    fn update(&mut self, message: Message, closing: bool) -> Task<Message> {
        let task = self.handle(message, closing);
        self.notify();
        task
    }

    /// Turns what the model reported into notifications.
    fn notify(&mut self) {
        for report in self.model.take_reports() {
            self.notifications.report(report, &self.model);
        }
    }

    fn handle(&mut self, message: Message, closing: bool) -> Task<Message> {
        let model = &mut self.model;
        let request = match message {
            Message::Model(message) => return model.update(message).map(Message::Model),
            Message::DismissNotification(id) => {
                self.notifications.dismiss(id);
                None
            }
            Message::CopyNotification(id) => {
                return match self.notifications.get(id) {
                    Some(notification) => iced::clipboard::write(format!(
                        "{}\n\n{}",
                        notification.title, notification.message
                    )),
                    None => Task::none(),
                };
            }
            Message::Picked(pick, Some(file)) => {
                let windows = file.extension().is_some_and(|extension| {
                    WINDOWS_PROGRAMS
                        .iter()
                        .any(|known| extension.eq_ignore_ascii_case(known))
                });
                if !windows {
                    let name = file.file_name().map_or_else(
                        || file.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    );
                    self.notifications.error(format!(
                        "{name} isn’t a Windows program. Pick the program’s .exe \
                         file, and extract the archive first if it came in one."
                    ));
                    return Task::none();
                }
                match pick {
                    Pick::AddProgram => {
                        if let Some(adding) = &mut self.adding {
                            adding.picked(file);
                        }
                        None
                    }
                    Pick::Run(prefix) => Some(Request::Act(Action::Run {
                        prefix,
                        executable: file,
                    })),
                }
            }
            Message::Picked(_, None) => None,
            Message::ExpandNotifications => {
                self.notifications.toggle_expanded();
                None
            }
            Message::DismissAllNotifications => {
                self.notifications.dismiss_all();
                None
            }
            _ if closing => None,
            Message::FollowNotification(id) => {
                let follow = self
                    .notifications
                    .get(id)
                    .and_then(|notification| notification.action.clone());
                self.notifications.dismiss(id);
                match follow.map(|(_, follow)| follow) {
                    Some(Follow::Act(action)) => Some(Request::Act(action)),
                    Some(Follow::Plugins) => {
                        self.preferences = Some(preferences::State::at(preferences::Page::Plugins));
                        None
                    }
                    None => None,
                }
            }
            Message::Onboarding(message) => match &mut self.screen {
                Screen::Onboarding(state) => state.update(message),
                _ => None,
            },
            Message::Classic(message) => match &mut self.screen {
                Screen::Classic(state) => state.update(model, message),
                _ => None,
            },
            Message::Next(message) => match &mut self.screen {
                Screen::Next(state) => state.update(message),
                _ => None,
            },
            Message::Preferences(message) => self
                .preferences
                .as_mut()
                .and_then(|preferences| preferences.update(message)),
            Message::Profiles(message) => self.profiles.update(model, message),
            Message::Adding(message) => match self
                .adding
                .as_mut()
                .and_then(|adding| adding.update(message))
            {
                None => None,
                Some(add_program::Event::PickFile) => return pick_program(Pick::AddProgram),
                Some(add_program::Event::Close) => {
                    self.adding_open = false;
                    None
                }
                Some(add_program::Event::Act(action)) => {
                    self.adding_open = false;
                    Some(Request::Act(action))
                }
            },
            Message::Booted(_)
            | Message::ThemeChanged(_)
            | Message::Window(_)
            | Message::ShutdownFinished(_) => unreachable!(),
        };
        match request {
            Some(request) => self.request(request),
            None => Task::none(),
        }
    }

    fn request(&mut self, request: Request) -> Task<Message> {
        match request {
            Request::Act(action) => {
                if let Action::SwitchExperience(experience) = action {
                    // The panes belong to the screen they were opened from.
                    self.screen = Screen::new(experience);
                    self.profiles = manage_profiles::State::default();
                    self.adding = None;
                    self.adding_open = false;
                }
                return self.model.perform(action).map(Message::Model);
            }
            Request::OpenPreferences => {
                self.preferences = Some(preferences::State::default());
            }
            Request::ClosePreferences => self.preferences = None,
            Request::AddProgram(target) => {
                self.adding = Some(add_program::State::new(target));
                self.adding_open = true;
            }
            Request::RunProgram(prefix) => return pick_program(Pick::Run(prefix)),
            Request::Control(Control::Profiles(message)) => {
                if let Some(request) = self.profiles.update(&self.model, message) {
                    return self.request(request);
                }
            }
            Request::Control(Control::CancelDownload(id)) => {
                return self.request(Request::Act(Action::CancelDownload(id)));
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let model = &self.model;
        if let Some(preferences) = &self.preferences {
            return preferences.view(model).map(Message::Preferences);
        }
        // Nothing during onboarding opens a pane.
        if let Screen::Onboarding(state) = &self.screen {
            return state.view(model).map(Message::Onboarding);
        }
        side_panel(
            Side::End,
            self.profiles.is_open(),
            move |_| {
                side_panel(
                    Side::Start,
                    self.adding_open,
                    move |pane| self.screen(pane),
                    move |pane| match &self.adding {
                        Some(adding) => adding.view(model, pane).map(Message::Adding),
                        None => iced::widget::column![].into(),
                    },
                )
            },
            move |pane| self.profiles.pane(model, pane).map(Message::Profiles),
        )
    }

    fn screen(&self, pane: PaneContext) -> Element<'_, Message> {
        let model = &self.model;
        match &self.screen {
            Screen::Onboarding(state) => state.view(model).map(Message::Onboarding),
            Screen::Classic(state) => state.view(model, pane).map(Message::Classic),
            Screen::Next(state) => state.view(model, pane).map(Message::Next),
        }
    }
}

/// Opens the system file picker for a Windows program.
fn pick_program(pick: Pick) -> Task<Message> {
    let title = match pick {
        Pick::AddProgram => "Add a Program",
        Pick::Run(_) => "Run a Program",
    };
    Task::perform(
        rfd::AsyncFileDialog::new()
            .set_title(title)
            .add_filter("Windows programs", &WINDOWS_PROGRAMS)
            .add_filter("All files", &["*"])
            .pick_file(),
        move |file| Message::Picked(pick, file.map(|file| file.path().to_owned())),
    )
}

fn shutdown(model: &Model) -> Task<Message> {
    Task::perform(model.shutdown(), Message::ShutdownFinished)
}

fn status<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    use iced::widget::{column, container, text};
    let header = chrome::header(true, |header| header);
    column![
        header,
        container(column![text(title).size(text_size::DISPLAY), text(description)].spacing(12))
            .center_x(Fill)
            .center_y(Fill)
    ]
    .height(Fill)
    .into()
}
