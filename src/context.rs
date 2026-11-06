use std::{collections::HashMap, sync::Arc};

use bottles_core::{AddonRole, Bottles, CatalogEntry};
use bottles_plugin_host::{Library, Plugin};
use iced::Task;

use crate::{
    app::Message,
    collection::Collection,
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
    pub(crate) collection: Collection,
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
        let operation = self.core.addons().fetch::<AddonRole>(id).map(drop);
        self.jobs.start(Kind::Fetch(id), operation)
    }

    pub(crate) fn save_collection(&mut self) -> Task<Message> {
        let save = self
            .collection
            .clone()
            .save(self.core.directories().data_dir());
        self.jobs.save(save)
    }

    pub(crate) fn launch(&mut self, entry: Entry) -> Task<Message> {
        let kind = Kind::Launch(entry.clone());
        let (prefix, group, program) = match &entry {
            Entry::Bottle(bottle, program) => (
                *bottle,
                *program,
                self.collection
                    .bottles
                    .get(bottle)
                    .and_then(|bottle| bottle.programs.get(program)),
            ),
            #[cfg(feature = "fvs")]
            Entry::Program(id) => (*id, *id, self.collection.programs.get(id)),
            Entry::Plugin(plugin, id) => {
                let library = self.libraries[plugin].clone();
                let (plugin, id) = (plugin.clone(), id.clone());
                return self.jobs.perform(kind, async move {
                    library
                        .launch(&id)
                        .await
                        .map_err(|error| format!("{plugin}: {error}"))
                });
            }
        };
        let Some(spec) = program.map(|program| program.spec.clone()) else {
            self.notifications.error("This program no longer exists");
            return Task::none();
        };
        match self.core.prefixes().open(prefix) {
            Ok(prefix) => {
                let operation = prefix.launch(group, spec);
                self.jobs.start(kind, operation)
            }
            Err(error) => {
                self.notifications.error(error.to_string());
                Task::none()
            }
        }
    }
}
