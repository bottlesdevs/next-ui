//! First-run onboarding: mode picker, a short tutorial carousel, and a
//! runtime-download checklist driven directly by the in-process addon manager.

use std::sync::Arc;

use bottles_core::{Addons, CatalogEntry, Component, IndexEntry, Slot, error::Error as CoreError};
use iced::{
    Background, Border, Element, Fill, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    theme::Mode as ThemeMode,
    widget::{button, center, column, container, row, text},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    Experience,
    icons::Icon,
    operation::{self, Event as OperationEvent, Outcome},
    theme,
    ui::chrome,
    widgets::{
        action_row::{ActionRow, State as RowState},
        header_bar::HeaderBar,
        row_group::RowGroup,
        text::TextExt as _,
    },
};

const STEP_WIDTH: f32 = 900.0;

const ONBOARDING_SLOTS: &[(Slot, &str)] =
    &[(Slot::WineBridge, "WineBridge"), (Slot::Runner, "Runner")];

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

struct DownloadItem {
    id: Uuid,
    label: String,
    size_label: String,
    progress: f32,
    state: DownloadState,
}

enum DownloadState {
    Running(CancellationToken),
    Succeeded,
    Cancelled,
    Unavailable,
    Failed(Arc<CoreError>),
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
    experience: Experience,
    addons: Addons,
    downloads: Vec<DownloadItem>,
    download_generation: u64,
    catalog_refresh: Option<CancellationToken>,
    catalog_error: Option<Arc<CoreError>>,
    system_theme: ThemeMode,
}

#[derive(Clone)]
pub enum Message {
    SelectExperience(Experience),
    ApplyExperience,
    NextTutorialStep,
    CatalogRefresh(OperationEvent<u64, ()>),
    Download(OperationEvent<(u64, Uuid), Arc<IndexEntry<Component>>>),
    CancelDownloads,
    Finished(Experience),
    Window(chrome::Action),
    SystemThemeChanged(ThemeMode),
}

impl State {
    pub fn new(addons: Addons) -> (Self, Task<Message>) {
        let state = Self {
            step: Step::Welcome,
            experience: Experience::Classic,
            addons,
            downloads: Vec::new(),
            download_generation: 0,
            catalog_refresh: None,
            catalog_error: None,
            system_theme: ThemeMode::default(),
        };
        let theme = iced::system::theme().map(Message::SystemThemeChanged);

        (state, theme)
    }

    pub fn theme(&self) -> Theme {
        theme::BottlesTheme::for_mode(self.system_theme).theme
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::system::theme_changes().map(Message::SystemThemeChanged)
    }

    pub fn cancel_active_operations(&self) {
        if let Some(cancellation) = &self.catalog_refresh {
            cancellation.cancel();
        }
        request_download_cancellation(&self.downloads);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectExperience(experience) => {
                if experience_available(experience) {
                    self.experience = experience;
                }
            }
            Message::ApplyExperience => self.step = Step::Tutorial(0),
            Message::NextTutorialStep => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIAL_STEPS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        self.step = Step::Downloads;
                        return self.start_setup();
                    }
                }
            }
            Message::CatalogRefresh(OperationEvent::Progress { .. }) => {}
            Message::CatalogRefresh(OperationEvent::Finished { key, outcome }) => {
                if key != self.download_generation {
                    return Task::none();
                }

                self.catalog_refresh = None;
                match outcome {
                    Outcome::Succeeded(()) => {}
                    Outcome::Cancelled => return Task::none(),
                    Outcome::Failed(error) => self.catalog_error = Some(error),
                }
                return self.start_downloads();
            }
            Message::Download(OperationEvent::Progress { key, progress }) => {
                if let Some(item) =
                    current_download_mut(&mut self.downloads, self.download_generation, key)
                    && matches!(&item.state, DownloadState::Running(_))
                {
                    if let Some(fraction) = progress.fraction() {
                        item.progress = fraction;
                    }
                    if let Some(total) = progress.transfer.and_then(|transfer| transfer.total) {
                        item.size_label = format_bytes(total);
                    }
                }
            }
            Message::Download(OperationEvent::Finished { key, outcome }) => {
                if let Some(item) =
                    current_download_mut(&mut self.downloads, self.download_generation, key)
                {
                    item.state = match outcome {
                        Outcome::Succeeded(_) => {
                            item.progress = 1.0;
                            DownloadState::Succeeded
                        }
                        Outcome::Cancelled => DownloadState::Cancelled,
                        Outcome::Failed(error) => DownloadState::Failed(error),
                    };
                }
            }
            Message::CancelDownloads => {
                self.cancel_active_operations();
                self.download_generation = self.download_generation.wrapping_add(1);
                self.catalog_refresh = None;
                self.step = Step::Welcome;
            }
            Message::Window(action) => return action.task().unwrap_or_else(Task::none),
            Message::Finished(_) => {}
            Message::SystemThemeChanged(mode) => self.system_theme = mode,
        }

        Task::none()
    }

    fn start_setup(&mut self) -> Task<Message> {
        self.cancel_active_operations();
        self.download_generation = self.download_generation.wrapping_add(1);
        let generation = self.download_generation;
        self.downloads.clear();
        self.catalog_error = None;

        let (cancellation, task) = operation::run(self.addons.refresh(), generation);
        self.catalog_refresh = Some(cancellation);

        task.map(Message::CatalogRefresh)
    }

    fn start_downloads(&mut self) -> Task<Message> {
        let generation = self.download_generation;
        let addons = self.addons.clone();
        let installed = addons.components();
        let catalog = addons.component_entries();
        let mut tasks = Vec::new();

        self.downloads.clear();

        for (slot, slot_label) in ONBOARDING_SLOTS {
            if let Some(entry) = installed.iter().find(|entry| entry.slot() == *slot) {
                self.downloads.push(DownloadItem {
                    id: entry.id(),
                    label: format!("{} {}", entry.name(), entry.version()),
                    size_label: (*slot_label).to_string(),
                    progress: 1.0,
                    state: DownloadState::Succeeded,
                });
                continue;
            }

            let Some(entry) = supported_component(&catalog, *slot) else {
                self.downloads.push(DownloadItem {
                    id: Uuid::new_v4(),
                    label: (*slot_label).to_string(),
                    size_label: "Unavailable".into(),
                    progress: 0.0,
                    state: DownloadState::Unavailable,
                });
                continue;
            };
            let id = entry.id();
            let (cancellation, task) = operation::run(addons.fetch_component(id), (generation, id));
            tasks.push(task.map(Message::Download));

            self.downloads.push(DownloadItem {
                id,
                label: format!("{} {}", entry.name(), entry.version()),
                size_label: (*slot_label).to_string(),
                progress: 0.0,
                state: DownloadState::Running(cancellation),
            });
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

        chrome::WindowFrame::new(
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
            self.experience_button(Experience::Next),
            self.experience_button(Experience::Classic),
        ]
        .spacing(12)
        .width(Length::FillPortion(1));

        let selector = row![experiences, self.selected_experience_view()]
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
        let mut failures = column![].spacing(8);
        let mut has_failures = false;

        if self.catalog_refresh.is_some() {
            group = group.add(
                ActionRow::new("Resource catalog", RowState::Progress(0.0))
                    .description("Refreshing"),
            );
        }

        for item in &self.downloads {
            let state = match &item.state {
                DownloadState::Running(_) => RowState::Progress(item.progress),
                DownloadState::Succeeded => RowState::Progress(1.0),
                DownloadState::Cancelled
                | DownloadState::Unavailable
                | DownloadState::Failed(_) => RowState::Disabled,
            };
            let description = match &item.state {
                DownloadState::Cancelled => "Cancelled",
                DownloadState::Unavailable => "Unavailable",
                DownloadState::Failed(_) => "Failed",
                DownloadState::Running(_) | DownloadState::Succeeded => &item.size_label,
            };

            group = group.add(ActionRow::new(&item.label, state).description(description));

            if let DownloadState::Failed(error) = &item.state {
                has_failures = true;
                failures = failures.push(
                    row![
                        Icon::Error.view(),
                        text(format!("{}: {error}", item.label)).detail(),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            } else if matches!(&item.state, DownloadState::Unavailable) {
                has_failures = true;
                failures = failures.push(
                    row![
                        Icon::Error.view(),
                        text(format!("{} is unavailable for this system", item.label)).detail(),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            }
        }

        if let Some(error) = &self.catalog_error {
            has_failures = true;
            failures = failures.push(
                row![
                    Icon::Error.view(),
                    text(format!("Could not refresh resource catalogs: {error}")).detail(),
                ]
                .spacing(8)
                .align_y(Vertical::Center),
            );
        }

        let cancel = pill_button("Cancel").on_press(Message::CancelDownloads);
        let done = pill_button("Get Started").on_press_maybe(
            downloads_complete(&self.downloads).then_some(Message::Finished(self.experience)),
        );

        let mut content = column![header, container(group).width(Fill)]
            .spacing(40)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center);
        if has_failures {
            content = content.push(container(failures).width(Fill));
        }
        content = content.push(row![cancel, done].spacing(12));

        center(content).into()
    }

    fn apply_button(&self) -> Element<'_, Message> {
        pill_button_with_icon("Apply Experience")
            .on_press(Message::ApplyExperience)
            .into()
    }

    fn experience_button(&self, experience: Experience) -> Element<'_, Message> {
        let selected = experience == self.experience;
        let mut title_row = row![text(experience_label(experience)).supporting()]
            .spacing(10)
            .align_y(Vertical::Center);

        if experience == Experience::Next {
            title_row = title_row.push(text("(coming later)").detail().muted());
        }

        let content = column![
            title_row,
            text(experience_caption(experience)).body().muted()
        ]
        .spacing(6);

        let button = button(
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
        });
        let button = if experience_available(experience) {
            button.on_press(Message::SelectExperience(experience))
        } else {
            button
        };

        button.padding(18).width(Fill).into()
    }

    fn selected_experience_view(&self) -> Element<'_, Message> {
        let title = row![
            Icon::Wand.view(),
            text(experience_label(self.experience)).supporting()
        ]
        .spacing(10)
        .align_y(Vertical::Center);
        let (first, second) = experience_detail(self.experience);
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

fn experience_available(experience: Experience) -> bool {
    experience == Experience::Classic
}

fn downloads_complete(downloads: &[DownloadItem]) -> bool {
    downloads.len() == ONBOARDING_SLOTS.len()
        && downloads
            .iter()
            .all(|item| matches!(&item.state, DownloadState::Succeeded))
}

fn current_download_mut(
    downloads: &mut [DownloadItem],
    current_generation: u64,
    (generation, id): (u64, Uuid),
) -> Option<&mut DownloadItem> {
    if generation != current_generation {
        return None;
    }
    downloads.iter_mut().find(|item| item.id == id)
}

fn request_download_cancellation(downloads: &[DownloadItem]) {
    for item in downloads {
        if let DownloadState::Running(cancellation) = &item.state {
            cancellation.cancel();
        }
    }
}

fn supported_component(
    entries: &[CatalogEntry<Component>],
    slot: Slot,
) -> Option<&CatalogEntry<Component>> {
    entries
        .iter()
        .find(|entry| entry.slot() == slot && entry.is_supported())
}

fn experience_label(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "Next Mode",
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
            "This experience is not available yet.",
        ),
        Experience::Classic => (
            "The software and games you install will be managed by Bottles in multiple environments.",
            "This gives advanced users the ability to fine-tune their experience.",
        ),
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
            row![icon, text_block].spacing(48).align_y(Vertical::Center),
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
        row![
            text(label).label(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn download(progress: f32, state: DownloadState) -> DownloadItem {
        DownloadItem {
            id: Uuid::nil(),
            label: String::new(),
            size_label: String::new(),
            progress,
            state,
        }
    }

    #[test]
    fn only_classic_is_available() {
        assert!(experience_available(Experience::Classic));
        assert!(!experience_available(Experience::Next));
    }

    #[test]
    fn component_selection_skips_unsupported_entries() {
        fn entry(id: &str, platform: Option<(&str, &str)>) -> CatalogEntry<Component> {
            let mut artifact = serde_json::json!({
                "url": "https://example.com/addon.tar.xz",
                "file_name": "addon.tar.xz",
                "checksum": { "algorithm": "sha256", "value": "00" },
            });
            if let Some((os, arch)) = platform {
                artifact["platform"] = serde_json::json!({ "os": os, "arch": arch });
            }

            serde_json::from_value(serde_json::json!({
                "id": id,
                "name": "WineBridge",
                "version": "1.0.0",
                "artifacts": [artifact],
                "slot": "winebridge",
            }))
            .unwrap()
        }

        let other_os = if cfg!(target_os = "linux") {
            "mac-os"
        } else {
            "linux"
        };
        let unsupported = entry(
            "77e90211-9091-47a9-bb00-0da6c2360981",
            Some((other_os, "x86_64")),
        );
        let universal = entry("d87fd8b8-8230-4e64-a66f-8b0e1c70c694", None);
        let entries = [unsupported.clone(), universal.clone()];

        assert_eq!(
            supported_component(&entries, Slot::WineBridge).map(CatalogEntry::id),
            Some(universal.id())
        );
        assert!(supported_component(&[unsupported], Slot::WineBridge).is_none());
    }

    #[test]
    fn setup_must_finish_successfully_before_continuing() {
        assert!(!downloads_complete(&[download(
            0.9,
            DownloadState::Running(CancellationToken::new()),
        )]));
        assert!(!downloads_complete(&[download(
            1.0,
            DownloadState::Cancelled,
        )]));
        assert!(!downloads_complete(&[download(
            0.0,
            DownloadState::Unavailable,
        )]));
        let succeeded = (0..ONBOARDING_SLOTS.len())
            .map(|_| download(1.0, DownloadState::Succeeded))
            .collect::<Vec<_>>();
        assert!(downloads_complete(&succeeded));
    }

    #[test]
    fn stale_download_keys_are_ignored() {
        let id = Uuid::new_v4();
        let mut downloads = vec![DownloadItem {
            id,
            label: String::new(),
            size_label: String::new(),
            progress: 0.0,
            state: DownloadState::Running(CancellationToken::new()),
        }];

        assert!(current_download_mut(&mut downloads, 2, (1, id)).is_none());
        assert!(current_download_mut(&mut downloads, 2, (2, id)).is_some());
    }

    #[test]
    fn cancellation_is_requested_without_discarding_the_running_state() {
        let cancellation = CancellationToken::new();
        let downloads = [download(0.5, DownloadState::Running(cancellation.clone()))];

        request_download_cancellation(&downloads);

        assert!(cancellation.is_cancelled());
        assert!(matches!(&downloads[0].state, DownloadState::Running(_)));
    }
}
