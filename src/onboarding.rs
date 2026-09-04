//! First-run onboarding: mode picker, a short tutorial carousel, and a
//! runtime-download checklist driven directly by the in-process addon manager.

mod setup;

use crate::{
    Experience, Icon, theme,
    widget::{
        ActionRow, Button, ButtonKind, HeaderBar, InfoCard, InfoCardKind, InfoRow, RowGroup,
        TextExt as _,
    },
    window,
};
use bottles_core::Addons;
use iced::{
    Element, Fill, Length, Task, Theme,
    alignment::{Horizontal, Vertical},
    widget::{column, container, row, scrollable, svg, text},
};

const EXPERIENCE_SELECTOR_MAX_WIDTH: f32 = 900.0;
const RESOURCE_LIST_MAX_WIDTH: f32 = 500.0;
const EXPERIENCE_ROW_GAP: f32 = 8.0;
const TUTORIAL_TEXT_MAX_WIDTH: f32 = 700.0;

struct TutorialStep {
    title: &'static str,
    body: &'static str,
}

const TUTORIAL_STEPS: &[TutorialStep] = &[
    TutorialStep {
        title: "What's a Bottle?",
        body: "Windows software in {OS} are not installed in the system, those lives in bottles. A bottle is a Windows-like space which makes the software feel in a native Windows environment.",
    },
    TutorialStep {
        title: "What are Environments?",
        body: "When you create a bottle, you must choose an environment, which is a set of configurations and utilities applied to ensure compatibility with the kind of software you want to install, i.e. games, applications.",
    },
    TutorialStep {
        title: "First Bottle Matter",
        body: "The first time you create a bottle using an environment, it takes some minutes. Then the next bottles will takes just a few seconds as the first created will be used as a template for all the others.",
    },
];

fn os_label() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        other => other,
    }
}

enum Step {
    Welcome,
    Tutorial(usize),
    Downloads,
}

pub struct State {
    step: Step,
    experience: Experience,
    setup: setup::Coordinator,
}

#[derive(Clone)]
pub enum Action {
    SelectExperience(Experience),
    ApplyExperience,
    NextTutorialStep,
    Setup(setup::Action),
    Finished(Experience),
    Window(window::Action),
}

#[derive(Clone)]
pub enum Event {
    Setup(setup::Event),
}

impl State {
    pub fn new(addons: Addons) -> Self {
        Self {
            step: Step::Welcome,
            experience: Experience::Classic,
            setup: setup::Coordinator::new(addons),
        }
    }

    pub fn cancel_active_operations(&self) {
        self.setup.cancel_active_operations();
    }

    pub fn has_active_operations(&self) -> bool {
        self.setup.has_active_operations()
    }

    pub fn update_action(&mut self, action: Action) -> Task<Event> {
        match action {
            Action::SelectExperience(experience) => {
                if experience_available(experience) {
                    self.experience = experience;
                }
            }
            Action::ApplyExperience => {
                self.step = Step::Tutorial(0);
                return self.setup.prepare().map(Event::Setup);
            }
            Action::NextTutorialStep => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIAL_STEPS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        self.step = Step::Downloads;
                    }
                }
            }
            Action::Setup(action) => return self.setup.update_action(action).map(Event::Setup),
            Action::Window(action) => return action.task().unwrap_or_else(Task::none),
            Action::Finished(_) => {}
        }

        Task::none()
    }

    pub fn update_event(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Setup(event) => self.setup.update_event(event).map(Event::Setup),
        }
    }

    pub fn view(&self) -> Element<'_, Action> {
        let content = match &self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(*index),
            Step::Downloads => self.downloads_view(),
        };

        shell(content, Action::Window(window::Action::Drag))
    }

    fn welcome_view(&self) -> Element<'_, Action> {
        let header = onboarding_title(
            "Welcome",
            "Choose the experience, you can change this later.",
        );

        let experiences = column![
            self.experience_button(Experience::Next),
            self.experience_button(Experience::Classic),
        ]
        .spacing(EXPERIENCE_ROW_GAP)
        .width(Length::FillPortion(1));
        let selector = row![experiences, self.selected_experience_view()]
            .spacing(EXPERIENCE_ROW_GAP)
            .width(Fill)
            .height(Length::Shrink);

        column![
            column![header, selector]
                .spacing(72)
                .width(Fill)
                .max_width(EXPERIENCE_SELECTOR_MAX_WIDTH)
                .align_x(Horizontal::Center),
            self.apply_button()
        ]
        .spacing(96)
        .width(Fill)
        .align_x(Horizontal::Center)
        .into()
    }

    fn downloads_view(&self) -> Element<'_, Action> {
        let header = onboarding_title(
            "Almost Done",
            "Bottles need to download the following small resources to be ready.",
        );

        let mut group = RowGroup::new();
        let mut failures = column![].spacing(8);
        let mut has_failures = false;

        if matches!(self.setup.phase(), setup::Phase::Preparing(_)) {
            group =
                group.row(ActionRow::progress("Resource catalog", 0.0).description("Preparing"));
        }

        for item in self.setup.downloads() {
            let description = match &item.state {
                setup::DownloadState::Pending => &item.size_label,
                setup::DownloadState::Cancelled => "Cancelled",
                setup::DownloadState::Unavailable => "Unavailable",
                setup::DownloadState::Failed(_) => "Failed",
                setup::DownloadState::Running(_) | setup::DownloadState::Succeeded => {
                    &item.size_label
                }
            };

            group = match &item.state {
                setup::DownloadState::Running(_) => group
                    .row(ActionRow::progress(&item.label, item.progress).description(description)),
                setup::DownloadState::Succeeded => {
                    group.row(ActionRow::progress(&item.label, 1.0).description(description))
                }
                setup::DownloadState::Pending
                | setup::DownloadState::Cancelled
                | setup::DownloadState::Unavailable
                | setup::DownloadState::Failed(_) => {
                    group.row(InfoRow::new(&item.label).description(description))
                }
            };

            if let setup::DownloadState::Failed(error) = &item.state {
                has_failures = true;
                failures = failures.push(
                    row![
                        error_icon(),
                        text(format!("{}: {error}", item.label)).detail(),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            } else if matches!(&item.state, setup::DownloadState::Unavailable) {
                has_failures = true;
                failures = failures.push(
                    row![
                        error_icon(),
                        text(format!("{} is unavailable for this system", item.label)).detail(),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            }
        }

        if let Some(error) = self.setup.catalog_error() {
            has_failures = true;
            failures = failures.push(
                row![
                    error_icon(),
                    text(format!("Could not refresh resource catalogs: {error}")).detail(),
                ]
                .spacing(8)
                .align_y(Vertical::Center),
            );
        }

        let action: Element<'_, Action> = match self.setup.phase() {
            setup::Phase::Idle | setup::Phase::Preparing(_) => {
                onboarding_button("Preparing…").into()
            }
            setup::Phase::Ready => onboarding_button_with_icon("Download")
                .on_press(Action::Setup(setup::Action::StartDownloads))
                .into(),
            setup::Phase::Downloading => onboarding_button("Cancel")
                .on_press(Action::Setup(setup::Action::CancelDownloads))
                .into(),
            setup::Phase::Cancelling => onboarding_button("Cancelling…").into(),
            setup::Phase::Failed => onboarding_button_with_icon("Retry")
                .on_press(Action::Setup(setup::Action::Retry))
                .into(),
            setup::Phase::Complete => onboarding_button_with_icon("Get Started")
                .on_press(Action::Finished(self.experience))
                .into(),
        };

        let mut resources = column![group]
            .spacing(24)
            .width(Fill)
            .max_width(RESOURCE_LIST_MAX_WIDTH);
        if has_failures {
            resources = resources.push(failures);
        }

        column![
            column![header, resources]
                .spacing(72)
                .align_x(Horizontal::Center),
            action,
        ]
        .spacing(96)
        .align_x(Horizontal::Center)
        .into()
    }

    fn apply_button(&self) -> Element<'_, Action> {
        onboarding_button_with_icon("Apply Experience")
            .on_press(Action::ApplyExperience)
            .into()
    }

    fn experience_button(&self, experience: Experience) -> Element<'_, Action> {
        let selected = experience == self.experience;
        let action =
            experience_available(experience).then_some(Action::SelectExperience(experience));

        ActionRow::new(experience_option_label(experience), action)
            .description(experience_caption(experience))
            .selected(selected)
            .into()
    }

    fn selected_experience_view(&self) -> Element<'_, Action> {
        let (first, second) = experience_detail(self.experience);

        InfoCard::new(
            InfoCardKind::Hint,
            experience_label(self.experience),
            format!("{first}\n\n{second}"),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }
}

pub(crate) fn shell<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_drag: Message,
) -> Element<'a, Message> {
    let header = HeaderBar::new(on_drag).transparent(true);
    let panel = container(
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
    .clip(true);

    panel.into()
}

fn experience_available(experience: Experience) -> bool {
    experience == Experience::Classic
}

fn experience_label(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "Next Mode",
        Experience::Classic => "Classic Mode",
    }
}

fn experience_option_label(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "Next Mode (coming later)",
        Experience::Classic => "Classic Mode",
    }
}

fn experience_caption(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "The easiest way to use Bottles.",
        Experience::Classic => "The experience for advanced users.",
    }
}

fn experience_detail(experience: Experience) -> (&'static str, &'static str) {
    match experience {
        Experience::Next => (
            "The software and games you install will be managed by Bottles using a single environment.",
            "This is the most convenient way if you are a beginner.",
        ),
        Experience::Classic => (
            "The software and games you install will be managed by Bottles in multiple environments.",
            "This gives advanced users the ability to fine-tune their experience.",
        ),
    }
}

fn onboarding_title<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Action> {
    column![
        text(title)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(crate::theme::colors(theme).accent),
            }),
        text(subtitle).size(16).medium().muted(),
    ]
    .align_x(Horizontal::Center)
    .spacing(6)
    .into()
}

fn error_icon<'a>() -> svg::Svg<'a> {
    Icon::Error.view().style(|theme: &Theme, _| svg::Style {
        color: Some(crate::theme::colors(theme).danger),
    })
}

fn tutorial_view<'a>(index: usize) -> Element<'a, Action> {
    let step = &TUTORIAL_STEPS[index];
    let body = step.body.replace("{OS}", os_label());
    let icon = Icon::Bottles
        .view()
        .width(160)
        .height(160)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(crate::theme::colors(theme).accent),
        });
    let text_block = column![
        text(step.title)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(crate::theme::colors(theme).accent),
            }),
        text(body).body().muted()
    ]
    .spacing(20)
    .max_width(TUTORIAL_TEXT_MAX_WIDTH);
    let next = onboarding_button_with_icon("Next").on_press(Action::NextTutorialStep);

    column![
        row![icon, text_block].spacing(48).align_y(Vertical::Center),
        next,
    ]
    .spacing(96)
    .align_x(Horizontal::Center)
    .into()
}

fn onboarding_button<'a>(label: &'a str) -> Button<'a, Action> {
    Button::custom(text(label).label()).kind(ButtonKind::Primary)
}

fn onboarding_button_with_icon<'a>(label: &'a str) -> Button<'a, Action> {
    onboarding_button(label)
        .trailing_icon(Icon::Arrow)
        .icon_rotation(std::f32::consts::PI)
        .icon_size(18.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_classic_is_available() {
        assert!(experience_available(Experience::Classic));
        assert!(!experience_available(Experience::Next));
    }
}
