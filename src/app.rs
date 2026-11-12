//! The application shell: lifecycle, window, overlays, and the shown screen.

use std::fmt;

use iced::{Element, Fill, Subscription, Task, theme::Mode as ThemeMode, window};
use next_ui::{chrome, theme};
use uuid::Uuid;

use crate::{
    classic,
    model::{self, Action, Boot, Experience, Model},
    next,
    notifications::{self, Follow},
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
    Ready { model: Box<Model>, screen: Screen },
}

enum Screen {
    Onboarding(onboarding::State),
    Classic(classic::State),
    Next(next::State),
    /// Preferences, and the screen to return to.
    Preferences(preferences::State, Box<Screen>),
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
    Window(chrome::Action),
    OpenPreferences,
    ClosePreferences,
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
    ExpandNotifications,
    DismissAllNotifications,
    ShutdownFinished(Result<(), String>),
    Onboarding(onboarding::Message),
    Classic(classic::Message),
    Next(next::Message),
    Preferences(preferences::Message),
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
            Self::ExpandNotifications => "ExpandNotifications",
            Self::DismissAllNotifications => "DismissAllNotifications",
            Self::ShutdownFinished(_) => "ShutdownFinished",
            Self::Onboarding(_) => "Onboarding",
            Self::Classic(_) => "Classic",
            Self::Next(_) => "Next",
            Self::Preferences(_) => "Preferences",
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
            State::Ready { model, .. } => model.subscription().map(Message::Model),
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
                    State::Ready { model, .. } => return shutdown(model),
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
                    self.state = State::Ready {
                        model: Box::new(model),
                        screen,
                    };
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
                let State::Ready { model, screen } = &mut self.state else {
                    return Task::none();
                };
                let request = match message {
                    Message::Model(message) => {
                        return model.update(message).map(Message::Model);
                    }
                    Message::DismissNotification(id) => {
                        model.notifications.dismiss(id);
                        None
                    }
                    Message::CopyNotification(id) => {
                        return match model.notifications.get(id) {
                            Some(notification) => iced::clipboard::write(format!(
                                "{}\n\n{}",
                                notification.title, notification.message
                            )),
                            None => Task::none(),
                        };
                    }
                    Message::ExpandNotifications => {
                        model.notifications.toggle_expanded();
                        None
                    }
                    Message::DismissAllNotifications => {
                        model.notifications.dismiss_all();
                        None
                    }
                    _ if self.closing => None,
                    Message::FollowNotification(id) => {
                        let follow = model
                            .notifications
                            .get(id)
                            .and_then(|notification| notification.action.clone());
                        model.notifications.dismiss(id);
                        match follow.map(|(_, follow)| follow) {
                            Some(Follow::Act(action)) => Some(Request::Act(action)),
                            Some(Follow::Plugins) => {
                                open_preferences(screen, preferences::Page::Plugins);
                                None
                            }
                            None => None,
                        }
                    }
                    Message::Onboarding(message) => match screen {
                        Screen::Onboarding(state) => state.update(message),
                        _ => None,
                    },
                    Message::Classic(message) => match screen {
                        Screen::Classic(state) => state.update(model, message),
                        _ => None,
                    },
                    Message::Next(message) => match screen {
                        Screen::Next(state) => state.update(model, message),
                        _ => None,
                    },
                    Message::Preferences(message) => match screen {
                        Screen::Preferences(state, _) => state.update(message),
                        _ => None,
                    },
                    Message::Booted(_)
                    | Message::ThemeChanged(_)
                    | Message::Window(_)
                    | Message::ShutdownFinished(_) => unreachable!(),
                };
                match request {
                    Some(Request::Window(action)) => return Task::done(Message::Window(action)),
                    Some(Request::Act(action)) => {
                        if let Action::SwitchExperience(experience) = action {
                            match screen {
                                Screen::Preferences(_, back) => **back = Screen::new(experience),
                                screen => *screen = Screen::new(experience),
                            }
                        }
                        return model.perform(action).map(Message::Model);
                    }
                    Some(Request::OpenPreferences) => {
                        open_preferences(screen, preferences::Page::default());
                    }
                    Some(Request::ClosePreferences) => {
                        if let Screen::Preferences(_, back) =
                            std::mem::replace(screen, Screen::Classic(classic::State::new()))
                        {
                            *screen = *back;
                        }
                    }
                    None => {}
                }
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
                State::Ready { model, screen } => {
                    let content = match screen {
                        Screen::Onboarding(state) => state.view(model).map(Message::Onboarding),
                        Screen::Classic(state) => state.view(model).map(Message::Classic),
                        Screen::Next(state) => state.view(model).map(Message::Next),
                        Screen::Preferences(state, _) => {
                            state.view(model).map(Message::Preferences)
                        }
                    };
                    let mut layers = stack![content].width(Fill).height(Fill);
                    if !model.notifications.is_empty() {
                        layers = layers.push(notifications::layer(&model.notifications));
                    }
                    layers.into()
                }
            }
        };
        chrome::window_frame(body, Message::Window)
    }
}

/// Shows Preferences at `page`, over the current screen unless it's already
/// open.
fn open_preferences(screen: &mut Screen, page: preferences::Page) {
    match screen {
        Screen::Preferences(state, _) => *state = preferences::State::at(page),
        screen => {
            let back = std::mem::replace(screen, Screen::Classic(classic::State::new()));
            *screen = Screen::Preferences(preferences::State::at(page), Box::new(back));
        }
    }
}

fn shutdown(model: &Model) -> Task<Message> {
    Task::perform(model.shutdown(), Message::ShutdownFinished)
}

fn status<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    use iced::widget::{column, container, text};
    let header = chrome::header(Message::Window(chrome::Action::Drag), true, |header| header);
    column![
        header,
        container(column![text(title).size(32), text(description)].spacing(12))
            .center_x(Fill)
            .center_y(Fill)
    ]
    .height(Fill)
    .into()
}
