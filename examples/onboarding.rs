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
    Background, Border, Element, Fill, Length, Task, Theme,
    alignment::{Horizontal, Vertical},
    futures::{SinkExt as _, Stream, StreamExt as _, future},
    widget::{button, column, container, row, scrollable, text},
};
use next_ui::{
    components::{
        action_row::{ActionRow, State as RowState},
        header_bar::HeaderBar,
        row_group::RowGroup,
        text::TextExt as _,
        window_frame,
    },
    icons::Icon,
    theme,
};
use uuid::Uuid;

/// Every step's content is capped at this width so the window doesn't
/// visibly grow or shrink in extent as the user moves between them.
const STEP_WIDTH: f32 = 900.0;

/// The component slots a first run needs, in the order the mockup lists
/// them (runner, then the DXVK/VKD3D/LatencyFlex trio it depends on).
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

/// Wraps a step's content in a themed, horizontally-centered scrollable so
/// nothing gets clipped if a step's content is taller than the window.
fn scroll_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let content = container(content).width(Fill).padding(32).center_x(Fill);

    container(
        scrollable(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4).margin(12),
            ))
            .style(theme::scrollbar)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
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

    /// The two-paragraph detail shown in the selected-mode panel.
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
    /// Falls back to the slot name until the operation's first progress
    /// event reports a transfer total — there's no static size in the
    /// catalog metadata, only what the download itself reports live.
    size_label: String,
    progress: f32,
    failed: bool,
}

#[derive(Clone)]
struct DownloadUpdate {
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

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Welcome to Bottles Next")
        .theme(App::theme)
        .style(|_, current_theme| theme::application(current_theme))
        .window_size((820.0, 640.0))
        .centered()
        .decorations(false)
        .transparent(true)
        .run()
}

struct App {
    step: Step,
    mode: Mode,
    bottles: Option<Bottles>,
    downloads: Vec<DownloadItem>,
}

#[derive(Clone)]
enum Message {
    BottlesLoaded(Result<Arc<Bottles>, String>),
    SelectMode(Mode),
    ApplyExperience,
    NextTutorialStep,
    DownloadProgress(Uuid, Result<DownloadUpdate, String>),
    CancelDownloads,
    Window(window_frame::Action),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let state = Self {
            step: Step::Welcome,
            mode: Mode::Next,
            bottles: None,
            downloads: Vec::new(),
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

        (state, boot)
    }

    fn theme(&self) -> Theme {
        theme::theme()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BottlesLoaded(Ok(bottles)) => {
                // `Bottles` isn't `Clone` (it owns the downloader/context),
                // so unwrap the `Arc` back out now that boot is done — we
                // only needed it to satisfy `Task::perform`'s `Send + 'static`
                // future, not for shared ownership.
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
        }

        Task::none()
    }

    /// Skips components already present locally and kicks off one
    /// `Task::run` per remaining component, each streaming its own progress
    /// through to completion.
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

    fn view(&self) -> Element<'_, Message> {
        let header = HeaderBar::new(Message::Window).show_window_controls(true);
        let content = match &self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(*index),
            Step::Downloads => self.downloads_view(),
        };

        window_frame::WindowFrame::new(
            column![header, scroll_panel(content)]
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

        // Left column sizes naturally (two buttons); the right panel
        // matches that height via `Fill` instead of the other way around,
        // so the buttons never get stretched taller than their content.
        let experiences = column![
            self.experience_button(Mode::Next),
            self.experience_button(Mode::Classic),
        ]
        .spacing(12)
        .width(Length::FillPortion(1));

        let selector = row![experiences, self.selected_mode_view()]
            .spacing(20)
            .width(Fill);

        column![header, selector, self.apply_button()]
            .spacing(48)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center)
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

        let cancel = button(text("Cancel").label())
            .style(|theme: &Theme, _status| button::Style {
                background: Some(Background::Color(
                    theme.extended_palette().background.strongest.color,
                )),
                text_color: theme.palette().text,
                border: Border::default().rounded(12),
                ..button::Style::default()
            })
            .padding([16, 24])
            .on_press(Message::CancelDownloads);

        column![header, container(group).width(Fill), cancel]
            .spacing(40)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center)
            .into()
    }

    fn apply_button(&self) -> Element<'_, Message> {
        button(
            row![
                text("Apply Experience").label(),
                Icon::Arrow.rotated(std::f32::consts::PI),
            ]
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
            .height(Fill)
            .max_height(220)
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
    let next = button(
        row![
            text("Next").label(),
            Icon::Arrow.rotated(std::f32::consts::PI)
        ]
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
    .on_press(Message::NextTutorialStep);

    column![
        row![icon, text_block]
            .spacing(48)
            .align_y(Vertical::Center),
        container(next).center_x(Fill),
    ]
    .spacing(64)
    .width(STEP_WIDTH)
    .into()
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
