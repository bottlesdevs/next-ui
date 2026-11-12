//! First-run experience selection and downloads of the essential addons.

use iced::{
    Element, Fill, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{column, container, row, scrollable, svg, text},
};
use next_ui::{
    Icon, chrome, theme,
    widget::{
        ActionRow, Button, ButtonKind, InfoCard, InfoCardKind, InfoRow, RowGroup, TextExt as _,
        spacing,
    },
};

use crate::{
    app::Request,
    model::{Action, Experience, Model, REQUIRED, Subject, required_available},
};

const TUTORIALS: [(&str, &str); 3] = [
    (
        "What's a Bottle?",
        "Windows software in {OS} are not installed in the system, those lives in bottles. A bottle is a Windows-like space which makes the software feel in a native Windows environment.",
    ),
    (
        "What are Environments?",
        "When you create a bottle, you must choose an environment, which is a set of configurations and utilities applied to ensure compatibility with the kind of software you want to install, i.e. games, applications.",
    ),
    (
        "First Bottle Matter",
        "The first time you create a bottle using an environment, it takes some minutes. Then the next bottles will takes just a few seconds as the first created will be used as a template for all the others.",
    ),
];

#[derive(Clone, Copy)]
enum Step {
    Welcome,
    Tutorial(usize),
    Downloads,
}

pub struct State {
    step: Step,
    experience: Experience,
}

#[derive(Clone)]
pub enum Message {
    SelectExperience(Experience),
    ApplyExperience,
    NextTutorial,
    Refresh,
    Act(Action),
    Window(chrome::Action),
}

impl State {
    pub fn new() -> Self {
        Self {
            step: Step::Welcome,
            experience: Experience::Classic,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Window(action) => return Some(Request::Window(action)),
            Message::Act(action) => return Some(Request::Act(action)),
            Message::SelectExperience(experience) => self.experience = experience,
            Message::ApplyExperience => {
                if self.experience == Experience::Next {
                    return Some(self.downloads());
                }
                self.step = Step::Tutorial(0);
            }
            Message::NextTutorial => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIALS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        return Some(self.downloads());
                    }
                }
            }
            Message::Refresh => return Some(self.downloads()),
        }
        None
    }

    fn downloads(&mut self) -> Request {
        self.step = Step::Downloads;
        Request::Act(Action::RefreshCatalog)
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let content = match self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(index),
            Step::Downloads => self.downloads_view(model),
        };
        shell(content)
    }

    fn welcome_view(&self) -> Element<'_, Message> {
        let mut choices = column![].spacing(spacing::SM).width(Length::FillPortion(1));
        for experience in [Experience::Next, Experience::Classic] {
            choices = choices.push(
                ActionRow::new(
                    experience_label(experience),
                    experience
                        .is_available()
                        .then_some(Message::SelectExperience(experience)),
                )
                .description(match experience {
                    Experience::Next => "The easiest way to use Bottles.",
                    Experience::Classic => "The experience for advanced users.",
                })
                .selected(experience == self.experience),
            );
        }
        let detail = match self.experience {
            Experience::Next => "Find and launch your software and games in one library.",
            Experience::Classic => "Manage multiple bottles and fine-tune their settings.",
        };
        column![
            title(
                "Welcome",
                "Choose the experience, you can change this later."
            ),
            row![
                choices,
                InfoCard::new(
                    InfoCardKind::Hint,
                    experience_label(self.experience),
                    detail
                )
                .width(Fill)
                .height(Fill)
            ]
            .spacing(spacing::SM)
            .width(Fill),
            next_button("Apply Experience").on_press(Message::ApplyExperience),
        ]
        .spacing(72)
        .max_width(900)
        .align_x(Horizontal::Center)
        .into()
    }

    fn downloads_view(&self, model: &Model) -> Element<'_, Message> {
        let preparing = model.operations.get(&Subject::Catalog).is_some();
        let entries = model.essential_entries();
        let available = required_available(&entries);
        let mut group = RowGroup::new();
        let mut running = false;
        for (role, label) in REQUIRED.into_iter().zip(["Runner", "WineBridge"]) {
            if !entries.iter().any(|entry| entry.role() == role) {
                group = group.row(InfoRow::new(label).description(if preparing {
                    "Preparing…"
                } else {
                    "Unavailable for this system. Refresh the catalog to retry."
                }));
            }
        }
        for entry in &entries {
            let label = format!("{} {}", entry.name(), entry.version());
            if model.installed(entry) {
                group = group.row(ActionRow::progress(label, 1.0).description("Installed"));
            } else if let Some(fetch) = model.operations.get(&Subject::Addon(entry.id())) {
                running = true;
                group = group.row(
                    ActionRow::progress(
                        label,
                        fetch
                            .progress
                            .as_ref()
                            .and_then(bottles_core::Progress::fraction)
                            .unwrap_or(0.0),
                    )
                    .description(fetch.progress.as_ref().map_or_else(
                        || "Preparing…".into(),
                        |progress| progress.stage.to_string(),
                    )),
                );
            } else {
                group = group.row(InfoRow::new(label).description("Pending"));
            }
        }
        let action = if model.can_create_bottles() {
            next_button("Get Started")
                .on_press(Message::Act(Action::SwitchExperience(self.experience)))
        } else if preparing {
            button("Preparing…")
        } else if running {
            button("Cancel").on_press(Message::Act(Action::CancelEssentials))
        } else if available {
            next_button("Download").on_press(Message::Act(Action::FetchEssentials))
        } else {
            button("Refresh catalog").on_press(Message::Refresh)
        };
        column![
            title("Almost Done", "Bottles needs these resources to be ready."),
            container(group).width(Fill).max_width(500),
            action,
        ]
        .spacing(72)
        .align_x(Horizontal::Center)
        .into()
    }
}

pub(crate) fn shell<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let header = chrome::header(Message::Window(chrome::Action::Drag), true, |header| {
        header.transparent(true)
    });
    container(
        column![
            header,
            scrollable(container(content).center(Fill).padding(32))
                .width(Fill)
                .height(Fill)
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::panel)
    .clip(true)
    .into()
}

fn experience_label(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "Next Mode",
        Experience::Classic => "Classic Mode",
    }
}

fn title<'a>(heading: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(heading)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(theme::colors(theme).accent)
            }),
        text(subtitle).size(16).medium().muted(),
    ]
    .align_x(Horizontal::Center)
    .spacing(6)
    .into()
}

fn os_label() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        other => other,
    }
}

fn tutorial_view(index: usize) -> Element<'static, Message> {
    let (heading, body) = TUTORIALS[index];
    let body = body.replace("{OS}", os_label());
    let icon = Icon::Bottles
        .view()
        .width(160)
        .height(160)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme::colors(theme).accent),
        });
    column![
        row![
            icon,
            column![title(heading, ""), text(body).body().muted()]
                .spacing(20)
                .max_width(700)
        ]
        .spacing(48)
        .align_y(Vertical::Center),
        next_button("Next").on_press(Message::NextTutorial),
    ]
    .spacing(96)
    .align_x(Horizontal::Center)
    .into()
}

fn button(label: &str) -> Button<'_, Message> {
    Button::custom(text(label).label()).kind(ButtonKind::Primary)
}

fn next_button(label: &str) -> Button<'_, Message> {
    button(label)
        .trailing_icon(Icon::Arrow)
        .icon_rotation(std::f32::consts::PI)
        .icon_size(18.0)
}
