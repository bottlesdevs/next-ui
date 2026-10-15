use std::{fmt, sync::Arc};

use bottles_core::Bottles;
use bottles_plugin_host::{Account, Library, Plugins};
use iced::{Element, Fill, Subscription, Task, theme::Mode as ThemeMode, window};
use uuid::Uuid;

#[cfg(feature = "fvs")]
use crate::next;
use crate::{
    chrome, classic,
    context::Context,
    jobs::{self, Jobs},
    notifications::{self, Notifications},
    onboarding,
    settings::{Experience, Settings},
    store::{self, Store},
    theme,
};

pub(crate) struct App {
    theme: iced::Theme,
    state: State,
    closing: bool,
}

enum State {
    Loading,
    Failed(String),
    Ready { ctx: Box<Context>, screen: Screen },
}

pub(crate) enum Screen {
    Onboarding(onboarding::State),
    Classic(classic::State),
    #[cfg(feature = "fvs")]
    Next(next::State),
}

impl Screen {
    fn new(experience: Experience) -> Self {
        match experience {
            Experience::Classic => Self::Classic(classic::State::new()),
            #[cfg(feature = "fvs")]
            Experience::Next => Self::Next(next::State::new()),
            #[cfg(not(feature = "fvs"))]
            Experience::Next => Self::Classic(classic::State::new()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Boot {
    settings: Settings,
    core: Arc<Bottles>,
}

#[derive(Clone)]
pub(crate) enum Message {
    Booted(Result<Boot, String>),
    ThemeChanged(ThemeMode),
    Window(chrome::Action),
    Store(store::Update),
    Job(Uuid, jobs::Event),
    SwitchExperience(Experience),
    DismissNotification(Uuid),
    CopyNotification(Uuid),
    ShutdownFinished(Result<(), String>),
    Onboarding(onboarding::Message),
    Classic(classic::Message),
    #[cfg(feature = "fvs")]
    Next(next::Message),
}

impl fmt::Debug for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Booted(_) => "Booted",
            Self::ThemeChanged(_) => "ThemeChanged",
            Self::Window(_) => "Window",
            Self::Store(_) => "Store",
            Self::Job(_, _) => "Job",
            Self::SwitchExperience(_) => "SwitchExperience",
            Self::DismissNotification(_) => "DismissNotification",
            Self::CopyNotification(_) => "CopyNotification",
            Self::ShutdownFinished(_) => "ShutdownFinished",
            Self::Onboarding(_) => "Onboarding",
            Self::Classic(_) => "Classic",
            #[cfg(feature = "fvs")]
            Self::Next(_) => "Next",
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
                Task::perform(boot(), Message::Booted),
                iced::system::theme().map(Message::ThemeChanged),
            ]),
        )
    }

    pub(crate) fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            chrome::subscription().map(Message::Window),
            iced::system::theme_changes().map(Message::ThemeChanged),
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
                    State::Ready { ctx, .. } => return shutdown(ctx),
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
                    let watches = Store::watch(&boot.core);
                    let ctx = Context {
                        store: Store::new(&boot.core),
                        jobs: Jobs::default(),
                        notifications: Notifications::default(),
                        settings: boot.settings,
                        core: boot.core,
                    };
                    let screen = ctx
                        .settings
                        .experience
                        .map(Screen::new)
                        .unwrap_or_else(|| Screen::Onboarding(onboarding::State::new()));
                    let close = if self.closing {
                        shutdown(&ctx)
                    } else {
                        Task::none()
                    };
                    self.state = State::Ready {
                        ctx: Box::new(ctx),
                        screen,
                    };
                    return Task::batch([watches, close]);
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
                let State::Ready { ctx, screen } = &mut self.state else {
                    return Task::none();
                };
                match message {
                    Message::Store(update) => ctx.store.update(update),
                    Message::Job(id, event) => {
                        if let Some(error) = ctx.jobs.update(id, event) {
                            ctx.notifications.error(error);
                        }
                    }
                    Message::DismissNotification(id) => ctx.notifications.dismiss(id),
                    Message::CopyNotification(id) => {
                        if let Some(notification) = ctx.notifications.get(id) {
                            return iced::clipboard::write(format!(
                                "{}\n\n{}",
                                notification.title, notification.message
                            ));
                        }
                    }
                    _ if self.closing => {}
                    Message::SwitchExperience(experience) => {
                        ctx.settings.experience = Some(experience);
                        *screen = Screen::new(experience);
                        return ctx.jobs.save_settings(
                            ctx.settings.clone(),
                            ctx.core.directories().config_dir(),
                        );
                    }
                    Message::Onboarding(message) => {
                        if let Screen::Onboarding(state) = screen {
                            return state.update(ctx, message);
                        }
                    }
                    Message::Classic(message) => {
                        if let Screen::Classic(state) = screen {
                            return state.update(ctx, message);
                        }
                    }
                    #[cfg(feature = "fvs")]
                    Message::Next(message) => {
                        if let Screen::Next(state) = screen {
                            return state.update(ctx, message);
                        }
                    }
                    Message::Booted(_)
                    | Message::ThemeChanged(_)
                    | Message::Window(_)
                    | Message::ShutdownFinished(_) => unreachable!(),
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
                State::Ready { ctx, screen } => {
                    let content = match screen {
                        Screen::Onboarding(state) => state.view(ctx).map(Message::Onboarding),
                        Screen::Classic(state) => state.view(ctx).map(Message::Classic),
                        #[cfg(feature = "fvs")]
                        Screen::Next(state) => state.view(ctx).map(Message::Next),
                    };
                    let mut layers = stack![content].width(Fill).height(Fill);
                    if !ctx.notifications.is_empty() {
                        layers = layers.push(notifications::layer(&ctx.notifications));
                    }
                    layers.into()
                }
            }
        };
        chrome::window_frame(body, Message::Window)
    }
}

async fn boot() -> Result<Boot, String> {
    let core = Bottles::open(bottles_core::Config::default())
        .await
        .map(Arc::new)
        .map_err(|error| error.to_string())?;
    let result = async {
        let plugins = Plugins::new(core.directories()).map_err(|error| error.to_string())?;
        for manifest in plugins.list().map_err(|error| error.to_string())? {
            let plugin = plugins
                .load(&manifest.id)
                .await
                .map_err(|error| error.to_string())?;
            if let Some(account) = plugin.cast::<Account>() {
                core.profiles().register_provider(Arc::new(account));
            }
            if let Some(library) = plugin.cast::<Library>() {
                core.library().register_provider(Arc::new(library));
            }
        }
        let settings = Settings::load(core.directories().config_dir()).await?;
        Ok(Boot {
            settings,
            core: core.clone(),
        })
    }
    .await;
    if result.is_err()
        && let Err(error) = core.shutdown().await
    {
        tracing::error!(%error, "core shutdown failed during boot cleanup");
    }
    result
}

fn shutdown(ctx: &Context) -> Task<Message> {
    let jobs = ctx.jobs.shutdown();
    let core = ctx.core.clone();
    Task::perform(
        async move {
            jobs.await;
            core.shutdown().await.map_err(|error| error.to_string())
        },
        Message::ShutdownFinished,
    )
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
