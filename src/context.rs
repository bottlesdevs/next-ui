use std::{collections::HashMap, sync::Arc};

use bottles_core::{AddonKind, Bottles, CatalogEntry};
use bottles_plugin_host::{Library, Plugin};
use iced::Task;

use crate::{
    app::Message,
    jobs::{Jobs, Kind},
    library::Entry,
    notifications::Notifications,
    profiles::Profiles,
    settings::Settings,
    store::Store,
};

pub(crate) struct Context {
    pub(crate) core: Arc<Bottles>,
    pub(crate) libraries: HashMap<String, Arc<Plugin<Library>>>,
    pub(crate) settings: Settings,
    // No screen reads profiles yet.
    #[allow(dead_code)]
    pub(crate) profiles: Profiles,
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

    pub(crate) fn launch(&mut self, entry: Entry) -> Task<Message> {
        let kind = Kind::Launch(entry.clone());
        match entry {
            Entry::Bottle(bottle, program) => match self.core.bottles().open(bottle) {
                Ok(bottle) => return self.jobs.start(kind, bottle.launch_program(program)),
                Err(error) => self.notifications.error(error.to_string()),
            },
            #[cfg(feature = "fvs")]
            Entry::Program(id) => match self.core.programs().open(id) {
                Ok(program) => return self.jobs.start(kind, program.launch()),
                Err(error) => self.notifications.error(error.to_string()),
            },
            Entry::Plugin(plugin, id) => {
                let library = self.libraries[&plugin].clone();
                return self.jobs.perform(kind, async move {
                    library
                        .launch(&id)
                        .await
                        .map_err(|error| format!("{plugin}: {error}"))
                });
            }
        }
        Task::none()
    }
}
