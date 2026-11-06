use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use bottles_core::{AddonsState, Core, Inventory};
use bottles_plugin_host::{Library, Plugin};
use iced::Task;

use crate::{app::Message, library::Listing};

pub(crate) struct Store {
    pub(crate) prefixes: Inventory,
    pub(crate) plugins: BTreeMap<String, Listing>,
    pub(crate) addons: Arc<AddonsState>,
}

#[derive(Clone, Debug)]
pub(crate) enum Update {
    Prefixes(Inventory),
    Plugin(String, Listing),
    Addons(Arc<AddonsState>),
}

impl Store {
    pub(crate) fn new(core: &Core, libraries: &HashMap<String, Arc<Plugin<Library>>>) -> Self {
        Self {
            prefixes: core.prefixes().inventory(),
            plugins: libraries
                .keys()
                .map(|id| (id.clone(), Listing::Loading))
                .collect(),
            addons: core.addons().state(),
        }
    }

    pub(crate) fn watch(
        core: &Core,
        libraries: &HashMap<String, Arc<Plugin<Library>>>,
    ) -> Task<Message> {
        let listings = libraries.iter().map(|(id, library)| {
            let id = id.clone();
            let library = library.clone();
            Task::perform(async move { library.list_entries().await }, move |result| {
                Message::Store(Update::Plugin(
                    id.clone(),
                    match result {
                        Ok(entries) => Listing::Loaded(entries),
                        Err(error) => Listing::Failed(error.to_string()),
                    },
                ))
            })
        });
        let watches = Task::batch([
            Task::run(core.prefixes().watch(), |inventory| {
                Message::Store(Update::Prefixes(inventory))
            }),
            Task::run(core.addons().watch(), |addons| {
                Message::Store(Update::Addons(addons))
            }),
        ]);
        Task::batch(listings.chain([watches]))
    }

    pub(crate) fn update(&mut self, update: Update) {
        match update {
            Update::Prefixes(prefixes) => self.prefixes = prefixes,
            Update::Plugin(id, listing) => {
                self.plugins.insert(id, listing);
            }
            Update::Addons(addons) => self.addons = addons,
        }
    }
}
