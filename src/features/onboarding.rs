//! First-run onboarding: mode picker, a short tutorial carousel, and a
//! runtime-download checklist — driven directly off `bottles_core::Bottles`
//! (in-process, no RPC; there's nothing here that needs another process).
//!
//! `Operation<T>` (the download's handle) isn't `Clone`, and awaiting it to
//! completion consumes it — but `Operation::progress()` only borrows `&self`
//! momentarily to clone an internal `watch::Receiver` into an independent
//! stream, so it can be called before the operation is moved. Each download
//! is driven by a single `Task::run` over a stream that interleaves that
//! progress with the operation's own completion, combining
//! `iced::stream::try_channel` with `futures::future::join`, rather than a
//! separate `Subscription` per item.

use std::sync::Arc;

use bottles_core::{Addons, Bottles, Config as CoreConfig, Slot};
use iced::{
    Background, Border, Element, Fill, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    futures::{SinkExt as _, Stream, StreamExt as _, future},
    theme::Mode as ThemeMode,
    widget::{button, center, column, container, row, text},
};
use uuid::Uuid;

use crate::{
    widgets::{
        action_row::{ActionRow, State as RowState},
        header_bar::HeaderBar,
        row_group::RowGroup,
        text::TextExt as _,
        window_frame,
    },
    icons::Icon,
    theme,
};

const STEP_WIDTH: f32 = 900.0;

const ONBOARDING_SLOTS: &[(Slot, &str)] = &[
    (Slot::Runner, "Runner"),
    (Slot::Dxvk, "DXVK"),
    (Slot::Vkd3d, "VKD3D"),
    (Slot::LatencyFlex, "LatencyFlex"),
];

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Next,
    Classic,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Self::Next => "Next Mode",
            Self::Classic => "Classic Mode",
        }
    }

    fn caption(self) -> &'static str {
        match self {
            Self::Next => "The easiest way to use Bottles.",
            Self::Classic => "The experience for advanced users.",
        }
    }

    fn detail(self) -> (&'static str, &'static str) {
        match self {
            Self::Next => (
                "The software and games you install will be managed by Bottles using a single environment.",
                "This is the most convenient way if you are a beginner.",
            ),
            Self::Classic => (
                "The software and games you install will be managed by Bottles in multiple environments.",
                "This gives advanced users the ability to fine-tune their experience.",
            ),
        }
    }
}

enum Step {
    Welcome,
    Tutorial(usize),
    Downloads,
}

struct DownloadItem {
    id: Uuid,
    label: String,
    size_label: String,
    progress: f32,
    failed: bool,
}

#[derive(Clone)]
pub struct DownloadUpdate {
    fraction: f32,
    size_label: Option<String>,
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = bytes as f64;

    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else {
        format!("{:.0} KB", bytes / KB)
    }
}

pub struct State {
    step: Step,
    mode: Mode,
    bottles: Option<Bottles>,
    downloads: Vec<DownloadItem>,
    system_theme: ThemeMode,
}

#[derive(Clone)]
pub enum Message {
    BottlesLoaded(Result<Arc<Bottles>, String>),
    SelectMode(Mode),
    ApplyExperience,
    NextTutorialStep,
    DownloadProgress(Uuid, Result<DownloadUpdate, String>),
    CancelDownloads,
    Finished,
    Window(window_frame::Action),
    SystemThemeChanged(ThemeMode),
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        let state = Self {
            step: Step::Welcome,
            mode: Mode::Next,
            bottles: None,
            downloads: Vec::new(),
            system_theme: ThemeMode::default(),
        };
        let boot = Task::perform(
            async {
                Bottles::open(CoreConfig::default())
                    .await
                    .map(Arc::new)
                    .map_err(|err| err.to_string())
            },
            Message::BottlesLoaded,
        );
        let theme = iced::system::theme().map(Message::SystemThemeChanged);

        (state, Task::batch([boot, theme]))
    }

    pub fn theme(&self) -> Theme {
        theme::BottlesTheme::for_mode(self.system_theme).theme
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::system::theme_changes().map(Message::SystemThemeChanged)
    }

    pub fn take_bottles(&mut self) -> Option<Bottles> {
        self.bottles.take()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BottlesLoaded(Ok(bottles)) => {
                if let Ok(bottles) = Arc::try_unwrap(bottles) {
                    self.bottles = Some(bottles);
                }
            }
            Message::BottlesLoaded(Err(err)) => eprintln!("failed to open Bottles: {err}"),
            Message::SelectMode(mode) => self.mode = mode,
            Message::ApplyExperience => self.step = Step::Tutorial(0),
            Message::NextTutorialStep => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIAL_STEPS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        self.step = Step::Downloads;
                        return self.start_downloads();
                    }
                }
            }
            Message::DownloadProgress(id, Ok(update)) => {
                if let Some(item) = self.downloads.iter_mut().find(|item| item.id == id) {
                    item.progress = update.fraction;

                    if let Some(size_label) = update.size_label {
                        item.size_label = size_label;
                    }
                }
            }
            Message::DownloadProgress(id, Err(err)) => {
                eprintln!("failed to download component {id}: {err}");

                if let Some(item) = self.downloads.iter_mut().find(|item| item.id == id) {
                    item.failed = true;
                }
            }
            Message::CancelDownloads => self.step = Step::Welcome,
            Message::Window(action) => return action.task(),
            Message::Finished => {}
            Message::SystemThemeChanged(mode) => self.system_theme = mode,
        }

        Task::none()
    }

    fn start_downloads(&mut self) -> Task<Message> {
        let Some(bottles) = &self.bottles else {
            return Task::none();
        };
        let addons = bottles.addons().clone();
        let installed: Vec<Uuid> = addons.components().iter().map(|entry| entry.id()).collect();
        let mut tasks = Vec::new();

        self.downloads.clear();

        for (slot, slot_label) in ONBOARDING_SLOTS {
            let Some(entry) = addons
                .component_entries()
                .into_iter()
                .find(|entry| entry.slot() == *slot)
            else {
                continue;
            };
            let id = entry.id();
            let done = installed.contains(&id);

            self.downloads.push(DownloadItem {
                id,
                label: format!("{} {}", entry.name(), entry.version()),
                size_label: (*slot_label).to_string(),
                progress: if done { 1.0 } else { 0.0 },
                failed: false,
            });

            if !done {
                tasks.push(Task::run(
                    fetch_component(addons.clone(), id),
                    move |result| Message::DownloadProgress(id, result),
                ));
            }
        }

        Task::batch(tasks)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window).show_window_controls(true);
        let content = match &self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(*index),
            Step::Downloads => self.downloads_view(),
        };

        window_frame::WindowFrame::new(
            column![
                header,
                container(content).width(Fill).height(Fill).padding(32)
            ]
            .width(Fill)
            .height(Fill),
            Message::Window,
        )
        .into()
    }

    fn welcome_view(&self) -> Element<'_, Message> {
        let header = column![
            text("Welcome").h3(),
            text("Choose the experience, you can change this later.")
                .body()
                .muted(),
        ]
        .align_x(Horizontal::Center)
        .spacing(8);

        let experiences = column![
            self.experience_button(Mode::Next),
            self.experience_button(Mode::Classic),
        ]
        .spacing(12)
        .width(Length::FillPortion(1));

        let selector = row![experiences, self.selected_mode_view()]
            .spacing(20)
            .width(Fill);

        center(
            column![header, selector, self.apply_button()]
                .spacing(48)
                .width(STEP_WIDTH)
                .align_x(Horizontal::Center),
        )
        .into()
    }

    fn downloads_view(&self) -> Element<'_, Message> {
        let header = column![
            text("Almost Done").h4(),
            text("Bottles need to download the following small resources to be ready.")
                .body()
                .muted(),
        ]
        .align_x(Horizontal::Center)
        .spacing(8);

        let mut group = RowGroup::new();

        for item in &self.downloads {
            let state = if item.failed {
                RowState::Disabled
            } else {
                RowState::Progress(item.progress)
            };

            group = group.add(
                ActionRow::new(&item.label, state).description(if item.failed {
                    "Failed"
                } else {
                    &item.size_label
                }),
            );
        }

        let cancel = pill_button("Cancel").on_press(Message::CancelDownloads);
        let done = pill_button("Get Started").on_press(Message::Finished);

        center(
            column![
                header,
                container(group).width(Fill),
                row![cancel, done].spacing(12),
            ]
            .spacing(40)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center),
        )
        .into()
    }

    fn apply_button(&self) -> Element<'_, Message> {
        pill_button_with_icon("Apply Experience")
            .on_press(Message::ApplyExperience)
            .into()
    }

    fn experience_button(&self, mode: Mode) -> Element<'_, Message> {
        let selected = mode == self.mode;
        let mut title_row = row![text(mode.label()).supporting()]
            .spacing(10)
            .align_y(Vertical::Center);

        if mode == Mode::Next {
            title_row = title_row.push(text("(recommended)").detail().muted());
        }

        let content = column![title_row, text(mode.caption()).body().muted()].spacing(6);

        button(
            row![
                content,
                iced::widget::Space::new().width(Fill),
                Icon::Arrow.rotated(std::f32::consts::PI)
            ]
            .align_y(Vertical::Center),
        )
        .style(move |theme: &Theme, _status| {
            let background = if selected {
                theme.extended_palette().background.stronger
            } else {
                theme.extended_palette().background.weak
            };

            button::Style {
                background: Some(Background::Color(background.color)),
                text_color: theme.palette().text,
                border: Border::default().rounded(12),
                ..button::Style::default()
            }
        })
        .on_press(Message::SelectMode(mode))
        .padding(18)
        .width(Fill)
        .into()
    }

    fn selected_mode_view(&self) -> Element<'_, Message> {
        let title = row![Icon::Wand.view(), text(self.mode.label()).supporting()]
            .spacing(10)
            .align_y(Vertical::Center);
        let (first, second) = self.mode.detail();
        let description = column![text(first).body(), text(second).body()].spacing(16);

        container(column![title, description].spacing(16))
            .width(Fill)
            .padding(20)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(
                    theme.extended_palette().background.weakest.color,
                )),
                border: Border::default().rounded(12),
                ..container::Style::default()
            })
            .into()
    }
}

fn tutorial_view<'a>(index: usize) -> Element<'a, Message> {
    let step = &TUTORIAL_STEPS[index];
    let body = step.body.replace("{OS}", os_label());
    let icon = iced::widget::svg(Icon::Bottles.handle())
        .width(160)
        .height(160)
        .content_fit(iced::ContentFit::Contain);
    let text_block = column![text(step.title).h4(), text(body).body().muted()]
        .spacing(20)
        .width(Fill);
    let next = pill_button_with_icon("Next").on_press(Message::NextTutorialStep);

    center(
        column![
            row![icon, text_block]
                .spacing(48)
                .align_y(Vertical::Center),
            container(next).center_x(Fill),
        ]
        .spacing(64)
        .width(STEP_WIDTH),
    )
    .into()
}

fn pill_button<'a>(label: &'a str) -> iced::widget::Button<'a, Message> {
    button(text(label).label())
        .style(|theme: &Theme, _status| button::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strongest.color,
            )),
            text_color: theme.palette().text,
            border: Border::default().rounded(12),
            ..button::Style::default()
        })
        .padding([16, 24])
}

fn pill_button_with_icon<'a>(label: &'a str) -> iced::widget::Button<'a, Message> {
    button(
        row![text(label).label(), Icon::Arrow.rotated(std::f32::consts::PI)]
            .spacing(10)
            .align_y(Vertical::Center),
    )
    .style(|theme: &Theme, _status| button::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.strongest.color,
        )),
        text_color: theme.palette().text,
        border: Border::default().rounded(12),
        ..button::Style::default()
    })
    .padding([16, 24])
}

fn fetch_component(addons: Addons, id: Uuid) -> impl Stream<Item = Result<DownloadUpdate, String>> {
    iced::stream::try_channel(
        16,
        move |mut output: iced::futures::channel::mpsc::Sender<DownloadUpdate>| async move {
            let operation = addons.fetch_component(id);
            let mut progress_stream = operation.progress();
            let mut progress_output = output.clone();

            let report_progress = async move {
                while let Some(progress) = progress_stream.next().await {
                    let update = DownloadUpdate {
                        fraction: progress.fraction().unwrap_or(0.0),
                        size_label: progress
                            .transfer
                            .and_then(|transfer| transfer.total)
                            .map(format_bytes),
                    };
                    let _ = progress_output.send(update).await;
                }
            };

            let (result, ()) = future::join(operation, report_progress).await;

            result.map_err(|err| err.to_string())?;
            output
                .send(DownloadUpdate {
                    fraction: 1.0,
                    size_label: None,
                })
                .await
                .map_err(|_| "onboarding window closed".to_string())?;
            Ok(())
        },
    )
}
