//! Runtime catalog preparation and component downloads for onboarding.

use std::sync::Arc;

use bottles_core::{Addons, CatalogEntry, Component, IndexEntry, Slot, error::Error as CoreError};
use iced::Task;
use tokio_util::sync::CancellationToken;

use crate::operation::{self, Event as OperationEvent, Outcome};

const REQUIRED_SLOTS: &[(Slot, &str)] =
    &[(Slot::WineBridge, "WineBridge"), (Slot::Runner, "Runner")];

pub(super) struct DownloadItem {
    slot: Slot,
    id: Option<uuid::Uuid>,
    pub(super) label: String,
    pub(super) size_label: String,
    pub(super) progress: f32,
    pub(super) state: DownloadState,
}

pub(super) enum DownloadState {
    Pending,
    Running(CancellationToken),
    Succeeded,
    Cancelled,
    Unavailable,
    Failed(Arc<CoreError>),
}

pub(super) enum Phase {
    Idle,
    Preparing(CancellationToken),
    Ready,
    Downloading,
    Cancelling,
    Failed,
    Complete,
}

#[derive(Clone)]
pub(crate) enum Action {
    StartDownloads,
    CancelDownloads,
    Retry,
}

#[derive(Clone)]
pub(crate) enum Event {
    CatalogRefresh(OperationEvent<u64, ()>),
    Download(OperationEvent<(u64, Slot), Arc<IndexEntry<Component>>>),
}

pub(super) struct Coordinator {
    addons: Addons,
    downloads: Vec<DownloadItem>,
    generation: u64,
    phase: Phase,
    catalog_error: Option<Arc<CoreError>>,
}

impl Coordinator {
    pub(super) fn new(addons: Addons) -> Self {
        Self {
            addons,
            downloads: Vec::new(),
            generation: 0,
            phase: Phase::Idle,
            catalog_error: None,
        }
    }

    pub(super) fn phase(&self) -> &Phase {
        &self.phase
    }

    pub(super) fn downloads(&self) -> &[DownloadItem] {
        &self.downloads
    }

    pub(super) fn catalog_error(&self) -> Option<&Arc<CoreError>> {
        self.catalog_error.as_ref()
    }

    pub(super) fn has_active_operations(&self) -> bool {
        matches!(self.phase, Phase::Preparing(_))
            || self
                .downloads
                .iter()
                .any(|item| matches!(item.state, DownloadState::Running(_)))
    }

    pub(super) fn cancel_active_operations(&self) {
        if let Phase::Preparing(cancellation) = &self.phase {
            cancellation.cancel();
        }
        request_download_cancellation(&self.downloads);
    }

    pub(super) fn prepare(&mut self) -> Task<Event> {
        self.cancel_active_operations();
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.downloads.clear();
        self.catalog_error = None;

        let (cancellation, task) = operation::run(self.addons.refresh(), generation);
        self.phase = Phase::Preparing(cancellation);

        task.map(Event::CatalogRefresh)
    }

    pub(super) fn update_action(&mut self, action: Action) -> Task<Event> {
        match action {
            Action::StartDownloads => self.start_downloads(),
            Action::CancelDownloads => {
                if matches!(self.phase, Phase::Downloading) {
                    request_download_cancellation(&self.downloads);
                    self.phase = Phase::Cancelling;
                }
                Task::none()
            }
            Action::Retry => {
                if self
                    .downloads
                    .iter()
                    .any(|item| matches!(item.state, DownloadState::Unavailable))
                {
                    self.prepare()
                } else {
                    self.start_downloads()
                }
            }
        }
    }

    pub(super) fn update_event(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::CatalogRefresh(OperationEvent::Progress { .. }) => {}
            Event::CatalogRefresh(OperationEvent::Finished { key, outcome }) => {
                if key != self.generation {
                    return Task::none();
                }

                self.catalog_error = match outcome {
                    Outcome::Succeeded(()) | Outcome::Cancelled => None,
                    Outcome::Failed(error) => Some(error),
                };
                self.prepare_downloads();
            }
            Event::Download(OperationEvent::Progress { key, progress }) => {
                if let Some(item) = current_download_mut(&mut self.downloads, self.generation, key)
                    && matches!(&item.state, DownloadState::Running(_))
                {
                    update_download_progress(item, &progress);
                }
            }
            Event::Download(OperationEvent::Finished { key, outcome }) => {
                if let Some(item) = current_download_mut(&mut self.downloads, self.generation, key)
                {
                    item.state = match outcome {
                        Outcome::Succeeded(_) => {
                            item.progress = 1.0;
                            DownloadState::Succeeded
                        }
                        Outcome::Cancelled => DownloadState::Cancelled,
                        Outcome::Failed(error) => DownloadState::Failed(error),
                    };

                    settle_download_batch(&mut self.phase, &mut self.downloads);
                }
            }
        }

        Task::none()
    }

    fn prepare_downloads(&mut self) {
        self.downloads =
            build_download_items(&self.addons.components(), &self.addons.component_entries());

        if downloads_complete(&self.downloads) {
            self.phase = Phase::Complete;
            self.catalog_error = None;
        } else if self
            .downloads
            .iter()
            .all(|item| !matches!(item.state, DownloadState::Unavailable))
        {
            self.phase = Phase::Ready;
            self.catalog_error = None;
        } else {
            self.phase = Phase::Failed;
        }
    }

    fn start_downloads(&mut self) -> Task<Event> {
        if !matches!(self.phase, Phase::Ready | Phase::Failed) {
            return Task::none();
        }

        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
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
            tasks.push(task.map(Event::Download));
            item.state = DownloadState::Running(cancellation);
        }

        if tasks.is_empty() {
            self.phase = if downloads_complete(&self.downloads) {
                Phase::Complete
            } else {
                Phase::Failed
            };
            Task::none()
        } else {
            self.catalog_error = None;
            self.phase = Phase::Downloading;
            Task::batch(tasks)
        }
    }
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

fn downloads_complete(downloads: &[DownloadItem]) -> bool {
    downloads.len() == REQUIRED_SLOTS.len()
        && downloads
            .iter()
            .all(|item| matches!(&item.state, DownloadState::Succeeded))
}

fn build_download_items(
    installed: &[Arc<IndexEntry<Component>>],
    catalog: &[CatalogEntry<Component>],
) -> Vec<DownloadItem> {
    REQUIRED_SLOTS
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

fn settle_download_batch(phase: &mut Phase, downloads: &mut [DownloadItem]) {
    if downloads
        .iter()
        .any(|item| matches!(item.state, DownloadState::Running(_)))
    {
        return;
    }

    if matches!(phase, Phase::Cancelling) {
        for item in &mut *downloads {
            if matches!(item.state, DownloadState::Cancelled) {
                item.state = DownloadState::Pending;
                item.size_label = "Ready to download".into();
            }
        }
    }

    *phase = if downloads_complete(downloads) {
        Phase::Complete
    } else if downloads
        .iter()
        .any(|item| matches!(item.state, DownloadState::Failed(_)))
    {
        Phase::Failed
    } else {
        Phase::Ready
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

        assert_eq!(downloads.len(), REQUIRED_SLOTS.len());
        for (item, (slot, _)) in downloads.iter().zip(REQUIRED_SLOTS) {
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
        let succeeded = REQUIRED_SLOTS
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
        let mut phase = Phase::Cancelling;
        let mut downloads = vec![
            download(
                Slot::WineBridge,
                DownloadState::Running(CancellationToken::new()),
            ),
            download(Slot::Runner, DownloadState::Cancelled),
        ];

        settle_download_batch(&mut phase, &mut downloads);
        assert!(matches!(phase, Phase::Cancelling));
        assert!(matches!(downloads[1].state, DownloadState::Cancelled));

        downloads[0].state = DownloadState::Succeeded;
        settle_download_batch(&mut phase, &mut downloads);

        assert!(matches!(phase, Phase::Ready));
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
