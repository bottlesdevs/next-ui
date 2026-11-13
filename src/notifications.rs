//! Session notifications and their overlay layer.

use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, mouse_area, responsive, row, scrollable, text},
};
use uuid::Uuid;

use next_ui::widget::{InfoCardKind, Notification as NotificationCard};

use crate::{
    app::Message,
    model::{Action, Model, Outcome, Refusal, Report, Subject, Verb},
};

/// How many notifications show before older ones fold away.
const VISIBLE: usize = 2;

/// What a notification's button does.
#[derive(Clone, Debug)]
pub(crate) enum Follow {
    Act(Action),
    /// Opens Preferences at Plugins.
    Plugins,
}

pub(crate) struct Notification {
    pub(crate) id: Uuid,
    pub(crate) kind: InfoCardKind,
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) action: Option<(String, Follow)>,
}

impl Notification {
    pub(crate) fn new(
        kind: InfoCardKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            title: title.into(),
            message: message.into(),
            action: None,
        }
    }

    /// Adds a button under the message, e.g. "Retry".
    pub(crate) fn action(mut self, label: impl Into<String>, follow: Follow) -> Self {
        self.action = Some((label.into(), follow));
        self
    }
}

/// The notifications of this session, oldest first.
#[derive(Default)]
pub(crate) struct Notifications {
    list: Vec<Notification>,
    /// Whether the folded older notifications are shown.
    expanded: bool,
}

impl Notifications {
    /// Tells of what the model reports, if it's news.
    pub(crate) fn report(&mut self, report: Report, model: &Model) {
        match report {
            Report::PluginFailed { id, error } => self.push(
                Notification::new(
                    InfoCardKind::Warning,
                    "A plugin couldn’t load",
                    format!("{id}: {error}"),
                )
                .action("Open Plugins", Follow::Plugins),
            ),
            Report::Finished {
                subject: Subject::Addon(release),
                verb: Verb::Fetch,
                then,
                result,
            } => {
                let name = model
                    .release_label(release)
                    .unwrap_or_else(|| "the release".to_owned());
                match (result, then) {
                    // Cancelling isn't news: the row the download was in
                    // shows it gone.
                    (Ok(Outcome::Cancelled), _) => {}
                    (Ok(_), Some(Action::UseRelease { .. })) => {
                        self.success(format!("{name} downloaded"), "It’s selected in the bottle.");
                    }
                    // Essentials show their progress where they were
                    // started, so finishing needs no notification.
                    (Ok(_), _) => {}
                    (Err(error), then) => {
                        let retry = match then {
                            Some(Action::UseRelease { prefix, release }) => {
                                Action::FetchRelease { prefix, release }
                            }
                            _ => Action::Download(release),
                        };
                        self.push(
                            Notification::new(
                                InfoCardKind::Error,
                                "Error",
                                format!("Couldn’t download {name}. {error}"),
                            )
                            .action("Retry", Follow::Act(retry)),
                        );
                    }
                }
            }
            Report::Finished {
                result: Err(error), ..
            }
            | Report::Error(error) => self.error(error),
            Report::Finished { .. } => {}
            Report::Refused(Refusal::NoRecipes(name)) => self.info(format!(
                "Installing {name} needs bottle recipes, which aren’t available yet"
            )),
            Report::Refused(Refusal::ReleaseGone) => {
                self.error("This release is no longer downloaded");
            }
            Report::Refused(Refusal::NoUmu) => {
                self.error("Proton runners need UMU, which isn't downloaded");
            }
            Report::Refused(Refusal::NoRunner) => {
                self.error("A runner is required to add a program");
            }
            Report::Refused(Refusal::NoRunnerOrWineBridge) => {
                self.error("A runner and WineBridge are required to create a bottle");
            }
            Report::Refused(Refusal::ProgramGone) => self.error("This program no longer exists"),
            Report::Refused(Refusal::PluginGone) => self.error("This plugin is no longer loaded"),
        }
    }

    fn info(&mut self, message: impl Into<String>) {
        self.push(Notification::new(
            InfoCardKind::Info,
            "Information",
            message,
        ));
    }

    fn success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push(Notification::new(InfoCardKind::Success, title, message));
    }

    pub(crate) fn error(&mut self, message: impl Into<String>) {
        self.push(Notification::new(InfoCardKind::Error, "Error", message));
    }

    fn push(&mut self, notification: Notification) {
        self.list.push(notification);
    }

    pub(crate) fn dismiss(&mut self, id: Uuid) {
        self.list.retain(|notification| notification.id != id);
        if self.list.len() <= VISIBLE {
            self.expanded = false;
        }
    }

    pub(crate) fn dismiss_all(&mut self) {
        self.list.clear();
        self.expanded = false;
    }

    pub(crate) fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub(crate) fn get(&self, id: Uuid) -> Option<&Notification> {
        self.list.iter().find(|notification| notification.id == id)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// The newest notifications in the bottom-right corner, newest last, under a
/// pill that unfolds the older ones or dismisses them all.
pub(crate) fn layer(notifications: &Notifications) -> Element<'_, Message> {
    responsive(move |size| {
        let list = &notifications.list;
        let folded = if notifications.expanded {
            0
        } else {
            list.len().saturating_sub(VISIBLE)
        };
        let cards = column(list[folded..].iter().map(card))
            .spacing(10)
            .width(Fill);
        let mut stack = column![].spacing(10).align_x(Alignment::End);
        if list.len() > VISIBLE {
            stack = stack.push(pill(folded));
        }
        let stack = stack.push(
            scrollable(cards)
                .anchor_bottom()
                .height(iced::Length::Shrink),
        );
        container(
            container(stack)
                .width(Fill)
                .max_width(420)
                .max_height(size.height * 0.6),
        )
        .width(Fill)
        .height(Fill)
        .align_right(Fill)
        .align_bottom(Fill)
        .padding(16)
        .into()
    })
    .width(Fill)
    .height(Fill)
    .into()
}

fn card(notification: &Notification) -> Element<'_, Message> {
    let mut card = NotificationCard::new(
        notification.kind,
        &notification.title,
        &notification.message,
    )
    .on_dismiss(Message::DismissNotification(notification.id));
    // Copying is for passing error details on, e.g. in a bug report.
    if notification.kind == InfoCardKind::Error {
        card = card.on_copy(Message::CopyNotification(notification.id));
    }
    if let Some((label, _)) = &notification.action {
        card = card.action(label, Message::FollowNotification(notification.id));
    }
    card.into()
}

/// "3 more | Dismiss All" while older notifications are folded, or
/// "Show Fewer | Dismiss All" once they're unfolded.
fn pill<'a>(folded: usize) -> Element<'a, Message> {
    let toggle = if folded > 0 {
        format!("{folded} more")
    } else {
        "Show Fewer".to_owned()
    };
    // Plain labels: a button's hover box would break up the pill.
    let label = |label: String, message| {
        mouse_area(text(label).size(15).style(|theme| text::Style {
            color: Some(next_ui::theme::colors(theme).accent_muted),
        }))
        .interaction(iced::mouse::Interaction::Pointer)
        .on_press(message)
    };
    container(
        row![
            label(toggle, Message::ExpandNotifications),
            container(Space::new().width(1).height(18)).style(|theme| {
                container::Style::default().background(next_ui::theme::colors(theme).selection)
            }),
            label("Dismiss All".to_owned(), Message::DismissAllNotifications),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .height(40)
    .padding([0, 16])
    .align_y(Alignment::Center)
    .style(|theme| {
        let colors = next_ui::theme::colors(theme);
        container::Style::default()
            .background(colors.border)
            .border(iced::Border::default().rounded(20))
    })
    .into()
}
