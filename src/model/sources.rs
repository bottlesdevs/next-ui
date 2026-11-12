//! Mirrors of the state the application reads from core and its plugins.

use std::{collections::BTreeMap, sync::Arc};

use bottles_core::{AddonsState, CatalogsState, Core, Inventory};
use bottles_plugin_host::{Library, LibraryEntry, Plugins};
use iced::Task;

pub(crate) struct Sources {
    pub(crate) prefixes: Inventory,
    pub(crate) addons: Arc<AddonsState>,
    pub(crate) catalogs: Arc<CatalogsState>,
    /// Library entries of each library plugin, keyed by plugin identifier.
    pub(crate) listings: BTreeMap<String, Listing>,
}

#[derive(Clone, Debug)]
pub(crate) enum Listing {
    Loading,
    Loaded(Vec<LibraryEntry>),
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) enum Update {
    Prefixes(Inventory),
    Addons(Arc<AddonsState>),
    Catalogs(Arc<CatalogsState>),
    Listing(String, Listing),
}

impl Sources {
    pub(crate) fn new(core: &Core, plugins: &Plugins) -> Self {
        Self {
            prefixes: core.prefixes().inventory(),
            addons: core.addons().state(),
            catalogs: core.catalogs().state(),
            listings: plugins
                .providing::<Library>()
                .iter()
                .map(|library| (library.manifest().id.clone(), Listing::Loading))
                .collect(),
        }
    }

    /// Follows every source for as long as the application runs.
    pub(crate) fn watch(core: &Core, plugins: &Plugins) -> Task<Update> {
        let listings = plugins.providing::<Library>().into_iter().map(|library| {
            let id = library.manifest().id.clone();
            Task::perform(async move { library.list_entries().await }, move |result| {
                Update::Listing(
                    id.clone(),
                    match result {
                        Ok(entries) => Listing::Loaded(entries),
                        Err(error) => Listing::Failed(error.to_string()),
                    },
                )
            })
        });
        let watches = Task::batch([
            Task::run(core.prefixes().watch(), Update::Prefixes),
            Task::run(core.addons().watch(), Update::Addons),
            Task::run(core.catalogs().watch(), Update::Catalogs),
        ]);
        Task::batch(listings.chain([watches]))
    }

    pub(crate) fn apply(&mut self, update: Update) {
        match update {
            Update::Prefixes(prefixes) => self.prefixes = prefixes,
            Update::Addons(addons) => self.addons = addons,
            Update::Catalogs(catalogs) => self.catalogs = catalogs,
            Update::Listing(plugin, listing) => {
                self.listings.insert(plugin, listing);
            }
        }
    }
}
