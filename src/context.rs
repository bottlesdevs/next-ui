use std::sync::Arc;

use bottles_core::{AddonKind, Bottles, CatalogEntry};
use iced::Task;

use crate::{
    app::Message,
    jobs::{Jobs, Kind},
    notifications::Notifications,
    settings::Settings,
    store::Store,
};

pub(crate) struct Context {
    pub(crate) core: Arc<Bottles>,
    pub(crate) settings: Settings,
    pub(crate) store: Store,
    pub(crate) jobs: Jobs,
    pub(crate) notifications: Notifications,
}

impl Context {
    pub(crate) fn fetch(&mut self, entry: CatalogEntry) -> Task<Message> {
        let id = entry.id();
        let addons = self.core.addons();
        let operation = match entry.kind() {
            AddonKind::Runner => addons.fetch_runner(id).map(drop),
            AddonKind::WineBridge => addons.fetch_winebridge(id).map(drop),
            AddonKind::Umu => addons.fetch_umu(id).map(drop),
            AddonKind::Component { .. } => addons.fetch_component(id).map(drop),
            AddonKind::Dependency => addons.fetch_dependency(id).map(drop),
        };
        self.jobs.start(Kind::Fetch(id), operation)
    }

    pub(crate) fn launch_entry(&mut self, provider: String, entry: String) -> Task<Message> {
        match self.core.library().launch(&provider, &entry) {
            Ok(operation) => self.jobs.start(Kind::Launch(provider, entry), operation),
            Err(error) => {
                self.notifications.error(error.to_string());
                Task::none()
            }
        }
    }
}
