//! First-run onboarding: mode picker, a short tutorial carousel, and a
//! runtime-download checklist driven directly by the in-process addon manager.

use std::sync::Arc;

use crate::{
    Experience,
    icons::Icon,
    operation::{self, Event as OperationEvent, Outcome},
    theme,
    ui::chrome,
    widgets::{
        action_row::{ActionRow, State as RowState},
        button::{Button, ButtonKind},
        header_bar::HeaderBar,
        info_card::{InfoCard, Kind as InfoCardKind},
        list_row::ListRow,
        row_group::RowGroup,
        text::TextExt as _,
    },
};
use bottles_core::{Addons, CatalogEntry, Component, IndexEntry, Slot, error::Error as CoreError};
use iced::{
    Element, Fill, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    theme::Mode as ThemeMode,
    widget::{center, column, container, row, text},
};
use tokio_util::sync::CancellationToken;

const STEP_WIDTH: f32 = 900.0;
const DOWNLOAD_WIDTH: f32 = 500.0;
const EXPERIENCE_ROW_GAP: f32 = 8.0;

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
    slot: Slot,
    id: Option<uuid::Uuid>,
    label: String,
    size_label: String,
    progress: f32,
    state: DownloadState,
}

enum DownloadState {
    Pending,
    Running(CancellationToken),
    Succeeded,
    Cancelled,
    Unavailable,
    Failed(Arc<CoreError>),
}

enum SetupPhase {
    Idle,
    Preparing(CancellationToken),
    Ready,
    Downloading,
    Cancelling,
    Failed,
    Complete,
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
    setup_phase: SetupPhase,
    catalog_error: Option<Arc<CoreError>>,
    system_theme: ThemeMode,
}

#[derive(Clone)]
pub enum Message {
    SelectExperience(Experience),
    ApplyExperience,
    NextTutorialStep,
    CatalogRefresh(OperationEvent<u64, ()>),
    StartDownloads,
    Download(OperationEvent<(u64, Slot), Arc<IndexEntry<Component>>>),
    CancelDownloads,
    Retry,
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
            setup_phase: SetupPhase::Idle,
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
        if let SetupPhase::Preparing(cancellation) = &self.setup_phase {
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
            Message::ApplyExperience => {
                self.step = Step::Tutorial(0);
                return self.start_preparation();
            }
            Message::NextTutorialStep => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIAL_STEPS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        self.step = Step::Downloads;
                    }
                }
            }
            Message::CatalogRefresh(OperationEvent::Progress { .. }) => {}
            Message::CatalogRefresh(OperationEvent::Finished { key, outcome }) => {
                if key != self.download_generation {
                    return Task::none();
                }

                self.catalog_error = match outcome {
                    Outcome::Succeeded(()) => None,
                    Outcome::Cancelled => None,
                    Outcome::Failed(error) => Some(error),
                };
                self.prepare_downloads();
            }
            Message::StartDownloads => return self.start_downloads(),
            Message::Download(OperationEvent::Progress { key, progress }) => {
                if let Some(item) =
                    current_download_mut(&mut self.downloads, self.download_generation, key)
                    && matches!(&item.state, DownloadState::Running(_))
                {
                    update_download_progress(item, &progress);
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

                    self.finish_download_batch();
                }
            }
            Message::CancelDownloads => {
                if matches!(self.setup_phase, SetupPhase::Downloading) {
                    request_download_cancellation(&self.downloads);
                    self.setup_phase = SetupPhase::Cancelling;
                }
            }
            Message::Retry => {
                if self
                    .downloads
                    .iter()
                    .any(|item| matches!(item.state, DownloadState::Unavailable))
                {
                    return self.start_preparation();
                }
                return self.start_downloads();
            }
            Message::Window(action) => return action.task().unwrap_or_else(Task::none),
            Message::Finished(_) => {}
            Message::SystemThemeChanged(mode) => self.system_theme = mode,
        }

        Task::none()
    }

    fn start_preparation(&mut self) -> Task<Message> {
        self.cancel_active_operations();
        self.download_generation = self.download_generation.wrapping_add(1);
        let generation = self.download_generation;
        self.downloads.clear();
        self.catalog_error = None;

        let (cancellation, task) = operation::run(self.addons.refresh(), generation);
        self.setup_phase = SetupPhase::Preparing(cancellation);

        task.map(Message::CatalogRefresh)
    }

    fn prepare_downloads(&mut self) {
        let installed = self.addons.components();
        let catalog = self.addons.component_entries();

        self.downloads = build_download_items(&installed, &catalog);

        if downloads_complete(&self.downloads) {
            self.setup_phase = SetupPhase::Complete;
            self.catalog_error = None;
        } else if self
            .downloads
            .iter()
            .all(|item| !matches!(item.state, DownloadState::Unavailable))
        {
            self.setup_phase = SetupPhase::Ready;
            self.catalog_error = None;
        } else {
            self.setup_phase = SetupPhase::Failed;
        }
    }

    fn start_downloads(&mut self) -> Task<Message> {
        if !matches!(self.setup_phase, SetupPhase::Ready | SetupPhase::Failed) {
            return Task::none();
        }

        self.download_generation = self.download_generation.wrapping_add(1);
        let generation = self.download_generation;
        let addons = self.addons.clone();
        let mut tasks = Vec::new();

        for item in &mut self.downloads {
            if matches!(item.state, DownloadState::Succeeded) {
                continue;
            }

            let Some(id) = item.id else {
                continue;
            };

            item.progress = 0.0;
            item.size_label = "Ready to download".into();
            let (cancellation, task) =
                operation::run(addons.fetch_component(id), (generation, item.slot));
            tasks.push(task.map(Message::Download));
            item.state = DownloadState::Running(cancellation);
        }

        if tasks.is_empty() {
            self.setup_phase = if downloads_complete(&self.downloads) {
                SetupPhase::Complete
            } else {
                SetupPhase::Failed
            };
            Task::none()
        } else {
            self.catalog_error = None;
            self.setup_phase = SetupPhase::Downloading;
            Task::batch(tasks)
        }
    }

    fn finish_download_batch(&mut self) {
        settle_download_batch(&mut self.setup_phase, &mut self.downloads);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match &self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(*index),
            Step::Downloads => self.downloads_view(),
        };

        frame(content, Message::Window)
    }

    fn welcome_view(&self) -> Element<'_, Message> {
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

        let details = container(self.selected_experience_view())
            .width(Length::FillPortion(1))
            .height(Fill)
            .clip(true)
            .style(|theme: &Theme| {
                let colors = theme::BottlesTheme::from(theme).hint;

                container::Style {
                    background: Some(iced::Background::Color(colors.color)),
                    border: iced::Border::default().rounded(6),
                    ..container::Style::default()
                }
            });
        let selector = row![experiences, details]
            .spacing(8)
            .width(Fill)
            .height(Length::Shrink);

        center(
            column![
                column![header, selector]
                    .spacing(72)
                    .width(Fill)
                    .align_x(Horizontal::Center),
                self.apply_button()
            ]
            .spacing(96)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center),
        )
        .into()
    }

    fn downloads_view(&self) -> Element<'_, Message> {
        let header = onboarding_title(
            "Almost Done",
            "Bottles need to download the following small resources to be ready.",
        );

        let mut group = RowGroup::new();
        let mut failures = column![].spacing(8);
        let mut has_failures = false;

        if matches!(self.setup_phase, SetupPhase::Preparing(_)) {
            group = group.add(
                ActionRow::new("Resource catalog", RowState::Progress(0.0))
                    .description("Preparing"),
            );
        }

        for item in &self.downloads {
            let description = match &item.state {
                DownloadState::Pending => &item.size_label,
                DownloadState::Cancelled => "Cancelled",
                DownloadState::Unavailable => "Unavailable",
                DownloadState::Failed(_) => "Failed",
                DownloadState::Running(_) | DownloadState::Succeeded => &item.size_label,
            };

            group = match &item.state {
                DownloadState::Running(_) => group.add(
                    ActionRow::new(&item.label, RowState::Progress(item.progress))
                        .description(description),
                ),
                DownloadState::Succeeded => group.add(
                    ActionRow::new(&item.label, RowState::Progress(1.0)).description(description),
                ),
                DownloadState::Pending
                | DownloadState::Cancelled
                | DownloadState::Unavailable
                | DownloadState::Failed(_) => group.add(ListRow::new(
                    column![
                        text(&item.label).label().medium(),
                        text(description).detail().muted()
                    ]
                    .spacing(6),
                )),
            };

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

        let action: Element<'_, Message> = match &self.setup_phase {
            SetupPhase::Idle | SetupPhase::Preparing(_) => onboarding_button("Preparing…").into(),
            SetupPhase::Ready => onboarding_button_with_icon("Download")
                .on_press(Message::StartDownloads)
                .into(),
            SetupPhase::Downloading => onboarding_button("Cancel")
                .on_press(Message::CancelDownloads)
                .into(),
            SetupPhase::Cancelling => onboarding_button("Cancelling…").into(),
            SetupPhase::Failed => onboarding_button_with_icon("Retry")
                .on_press(Message::Retry)
                .into(),
            SetupPhase::Complete => onboarding_button_with_icon("Get Started")
                .on_press(Message::Finished(self.experience))
                .into(),
        };

        let mut resources = column![container(group).width(DOWNLOAD_WIDTH).center_x(Fill)]
            .spacing(24)
            .align_x(Horizontal::Center);
        if has_failures {
            resources = resources.push(container(failures).width(DOWNLOAD_WIDTH));
        }

        center(
            column![
                column![header, resources]
                    .spacing(72)
                    .width(Fill)
                    .align_x(Horizontal::Center),
                action,
            ]
            .spacing(96)
            .width(STEP_WIDTH)
            .align_x(Horizontal::Center),
        )
        .into()
    }

    fn apply_button(&self) -> Element<'_, Message> {
        onboarding_button_with_icon("Apply Experience")
            .on_press(Message::ApplyExperience)
            .into()
    }

    fn experience_button(&self, experience: Experience) -> Element<'_, Message> {
        let selected = experience == self.experience;
        let state = if experience_available(experience) {
            RowState::Ready(Message::SelectExperience(experience))
        } else {
            RowState::Disabled
        };

        ListRow::from(
            ActionRow::new(experience_option_label(experience), state)
                .description(experience_caption(experience)),
        )
        .selected(selected)
        .into()
    }

    fn selected_experience_view(&self) -> Element<'_, Message> {
        let (first, second) = experience_detail(self.experience);

        InfoCard::new(
            InfoCardKind::Hint,
            experience_label(self.experience),
            format!("{first}\n\n{second}"),
        )
        .into()
    }
}

pub(crate) fn frame<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_action: fn(chrome::Action) -> Message,
) -> Element<'a, Message> {
    let header = HeaderBar::new(on_action)
        .show_window_controls(true)
        .transparent(true);
    let panel = container(
        column![
            header,
            container(center(content))
                .width(Fill)
                .height(Fill)
                .padding(32)
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::panel)
    .clip(true);

    chrome::WindowFrame::new(panel, on_action).into()
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

fn build_download_items(
    installed: &[Arc<IndexEntry<Component>>],
    catalog: &[CatalogEntry<Component>],
) -> Vec<DownloadItem> {
    ONBOARDING_SLOTS
        .iter()
        .map(|(slot, slot_label)| {
            if let Some(entry) = installed.iter().find(|entry| entry.slot() == *slot) {
                return DownloadItem {
                    slot: *slot,
                    id: Some(entry.id()),
                    label: format!("{} {}", entry.name(), entry.version()),
                    size_label: "Installed".into(),
                    progress: 1.0,
                    state: DownloadState::Succeeded,
                };
            }

            let Some(entry) = supported_component(catalog, *slot) else {
                return DownloadItem {
                    slot: *slot,
                    id: None,
                    label: (*slot_label).to_string(),
                    size_label: "Unavailable".into(),
                    progress: 0.0,
                    state: DownloadState::Unavailable,
                };
            };

            DownloadItem {
                slot: *slot,
                id: Some(entry.id()),
                label: format!("{} {}", entry.name(), entry.version()),
                size_label: "Ready to download".into(),
                progress: 0.0,
                state: DownloadState::Pending,
            }
        })
        .collect()
}

fn current_download_mut(
    downloads: &mut [DownloadItem],
    current_generation: u64,
    (generation, slot): (u64, Slot),
) -> Option<&mut DownloadItem> {
    if generation != current_generation {
        return None;
    }
    downloads.iter_mut().find(|item| item.slot == slot)
}

fn update_download_progress(item: &mut DownloadItem, progress: &bottles_core::Progress) {
    if let Some(fraction) = progress.fraction() {
        item.progress = fraction;
    }
    if let Some(total) = progress.transfer.and_then(|transfer| transfer.total) {
        item.size_label = format_bytes(total);
    }
}

fn settle_download_batch(phase: &mut SetupPhase, downloads: &mut [DownloadItem]) {
    if downloads
        .iter()
        .any(|item| matches!(item.state, DownloadState::Running(_)))
    {
        return;
    }

    if matches!(phase, SetupPhase::Cancelling) {
        for item in &mut *downloads {
            if matches!(item.state, DownloadState::Cancelled) {
                item.state = DownloadState::Pending;
                item.size_label = "Ready to download".into();
            }
        }
    }

    *phase = if downloads_complete(downloads) {
        SetupPhase::Complete
    } else if downloads
        .iter()
        .any(|item| matches!(item.state, DownloadState::Failed(_)))
    {
        SetupPhase::Failed
    } else {
        SetupPhase::Ready
    };
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

fn onboarding_title<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(title)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(theme.palette().primary),
            }),
        text(subtitle).size(16).medium().muted(),
    ]
    .align_x(Horizontal::Center)
    .spacing(6)
    .into()
}

fn tutorial_view<'a>(index: usize) -> Element<'a, Message> {
    let step = &TUTORIAL_STEPS[index];
    let body = step.body.replace("{OS}", os_label());
    let icon = iced::widget::svg(Icon::Bottles.handle())
        .width(160)
        .height(160)
        .content_fit(iced::ContentFit::Contain);
    let text_block = column![
        text(step.title)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(theme.palette().primary),
            }),
        text(body).body().muted()
    ]
    .spacing(20)
    .width(Fill);
    let next = onboarding_button_with_icon("Next").on_press(Message::NextTutorialStep);

    center(
        column![
            row![icon, text_block].spacing(48).align_y(Vertical::Center),
            container(next).center_x(Fill),
        ]
        .spacing(96)
        .width(STEP_WIDTH),
    )
    .into()
}

fn onboarding_button<'a>(label: &'a str) -> Button<'a, Message> {
    Button::new(text(label).label()).kind(ButtonKind::Primary)
}

fn onboarding_button_with_icon<'a>(label: &'a str) -> Button<'a, Message> {
    onboarding_button(label)
        .trailing_icon(Icon::Arrow)
        .icon_rotation(std::f32::consts::PI)
        .icon_size(18.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn download(slot: Slot, state: DownloadState) -> DownloadItem {
        DownloadItem {
            slot,
            id: Some(Uuid::nil()),
            label: String::new(),
            size_label: String::new(),
            progress: 0.0,
            state,
        }
    }

    fn entry(
        id: &str,
        name: &str,
        slot: Slot,
        platform: Option<(&str, &str)>,
    ) -> CatalogEntry<Component> {
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
            "name": name,
            "version": "1.0.0",
            "artifacts": [artifact],
            "slot": slot.as_str(),
        }))
        .unwrap()
    }

    #[test]
    fn only_classic_is_available() {
        assert!(experience_available(Experience::Classic));
        assert!(!experience_available(Experience::Next));
    }

    #[test]
    fn component_selection_skips_unsupported_entries() {
        let other_os = if cfg!(target_os = "linux") {
            "mac-os"
        } else {
            "linux"
        };
        let unsupported = entry(
            "77e90211-9091-47a9-bb00-0da6c2360981",
            "WineBridge",
            Slot::WineBridge,
            Some((other_os, "x86_64")),
        );
        let universal = entry(
            "d87fd8b8-8230-4e64-a66f-8b0e1c70c694",
            "WineBridge",
            Slot::WineBridge,
            None,
        );
        let entries = [unsupported.clone(), universal.clone()];

        assert_eq!(
            supported_component(&entries, Slot::WineBridge).map(CatalogEntry::id),
            Some(universal.id())
        );
        assert!(supported_component(&[unsupported], Slot::WineBridge).is_none());
    }

    #[test]
    fn prepared_catalog_entries_are_pending_not_running() {
        let catalog = [
            entry(
                "d87fd8b8-8230-4e64-a66f-8b0e1c70c694",
                "WineBridge",
                Slot::WineBridge,
                None,
            ),
            entry(
                "77e90211-9091-47a9-bb00-0da6c2360981",
                "Runner",
                Slot::Runner,
                None,
            ),
        ];

        let downloads = build_download_items(&[], &catalog);

        assert_eq!(downloads.len(), ONBOARDING_SLOTS.len());
        for (item, (slot, _)) in downloads.iter().zip(ONBOARDING_SLOTS) {
            assert_eq!(item.slot, *slot);
            assert!(item.id.is_some());
            assert_eq!(item.progress, 0.0);
            assert_eq!(item.size_label, "Ready to download");
            assert!(matches!(item.state, DownloadState::Pending));
        }
        assert!(
            downloads
                .iter()
                .all(|item| !matches!(item.state, DownloadState::Running(_)))
        );
    }

    #[test]
    fn setup_completes_only_when_every_required_download_succeeds() {
        assert!(!downloads_complete(&[download(
            Slot::WineBridge,
            DownloadState::Running(CancellationToken::new()),
        )]));
        assert!(!downloads_complete(&[download(
            Slot::WineBridge,
            DownloadState::Cancelled,
        )]));
        assert!(!downloads_complete(&[download(
            Slot::WineBridge,
            DownloadState::Unavailable,
        )]));
        let succeeded = ONBOARDING_SLOTS
            .iter()
            .map(|(slot, _)| download(*slot, DownloadState::Succeeded))
            .collect::<Vec<_>>();

        assert!(downloads_complete(&succeeded));
    }

    #[test]
    fn download_keys_match_generation_and_slot() {
        let mut downloads = vec![
            download(
                Slot::WineBridge,
                DownloadState::Running(CancellationToken::new()),
            ),
            download(
                Slot::Runner,
                DownloadState::Running(CancellationToken::new()),
            ),
        ];

        assert!(
            current_download_mut(&mut downloads, 2, (1, Slot::Runner)).is_none(),
            "a stale generation must not update any slot"
        );
        assert_eq!(
            current_download_mut(&mut downloads, 2, (2, Slot::Runner)).map(|item| item.slot),
            Some(Slot::Runner)
        );
        assert!(current_download_mut(&mut downloads, 2, (2, Slot::Dxvk)).is_none());
    }

    #[test]
    fn cancellation_is_requested_without_discarding_the_running_token() {
        let cancellation = CancellationToken::new();
        let downloads = [download(
            Slot::WineBridge,
            DownloadState::Running(cancellation.clone()),
        )];

        request_download_cancellation(&downloads);

        assert!(cancellation.is_cancelled());
        let DownloadState::Running(retained) = &downloads[0].state else {
            panic!("cancellation must retain the running state until its terminal event");
        };
        assert!(retained.is_cancelled());
    }

    #[test]
    fn cancelling_waits_for_every_terminal_event_then_restores_pending_rows() {
        let mut phase = SetupPhase::Cancelling;
        let mut downloads = vec![
            download(
                Slot::WineBridge,
                DownloadState::Running(CancellationToken::new()),
            ),
            download(Slot::Runner, DownloadState::Cancelled),
        ];

        settle_download_batch(&mut phase, &mut downloads);
        assert!(matches!(phase, SetupPhase::Cancelling));
        assert!(matches!(downloads[1].state, DownloadState::Cancelled));

        downloads[0].state = DownloadState::Succeeded;
        settle_download_batch(&mut phase, &mut downloads);

        assert!(matches!(phase, SetupPhase::Ready));
        assert!(matches!(downloads[0].state, DownloadState::Succeeded));
        assert!(matches!(downloads[1].state, DownloadState::Pending));
        assert_eq!(downloads[1].size_label, "Ready to download");
    }

    #[test]
    fn byte_sizes_use_binary_units() {
        assert_eq!(format_bytes(0), "0 KB");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(10 * 1024), "10 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 / 2), "1.5 MB");
    }

    #[test]
    fn download_progress_uses_the_reported_http_total() {
        let mut item = download(Slot::WineBridge, DownloadState::Pending);
        item.size_label = "Ready to download".into();
        let progress = bottles_core::Progress {
            stage: bottles_core::Stage::Downloading {
                file: "winebridge.tar.xz".into(),
            },
            transfer: Some(bottles_core::Transfer {
                current: 2 * 1024 * 1024,
                total: Some(4 * 1024 * 1024),
            }),
        };

        update_download_progress(&mut item, &progress);

        assert_eq!(item.progress, 0.5);
        assert_eq!(item.size_label, "4.0 MB");
    }
}
