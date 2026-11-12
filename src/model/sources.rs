//! Mirrors of the state the application reads from core and its plugins.

use std::{
    collections::{BTreeMap, HashMap},
    hash::{Hash, Hasher},
    sync::Arc,
};

use bottles_core::{AddonsState, CatalogsState, Core, Inventory, Prefix};
use bottles_plugin_host::{Library, LibraryEntry, Plugins};
use iced::{
    Subscription, Task,
    futures::{Stream, StreamExt as _},
};
use uuid::Uuid;

pub(crate) struct Sources {
    pub(crate) prefixes: Inventory,
    /// Process groups with a running process, for each prefix where Wine runs.
    pub(crate) running: HashMap<Uuid, Vec<Uuid>>,
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
    /// The running process groups of a prefix, or `None` once Wine stops.
    Groups(Uuid, Option<Vec<Uuid>>),
    Addons(Arc<AddonsState>),
    Catalogs(Arc<CatalogsState>),
    Listing(String, Listing),
}

impl Sources {
    pub(crate) fn new(core: &Core, plugins: &Plugins) -> Self {
        Self {
            prefixes: core.prefixes().inventory(),
            running: HashMap::new(),
            addons: core.addons().state(),
            catalogs: core.catalogs().state(),
            listings: plugins
                .providing::<Library>()
                .iter()
                .map(|library| (library.manifest().id.clone(), Listing::Loading))
                .collect(),
        }
    }

    /// Follows every source that lasts for as long as the application runs.
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

    /// Follows the running process groups of `prefixes`.
    ///
    /// A prefix is followed from the first call that names it until a call
    /// leaves it out.
    pub(crate) fn follow(
        core: &Core,
        prefixes: impl Iterator<Item = Uuid>,
    ) -> Subscription<Update> {
        Subscription::batch(
            prefixes
                .filter_map(|id| core.prefixes().open(id).ok())
                .map(|prefix| Subscription::run_with(Groups(prefix), groups)),
        )
    }

    pub(crate) fn apply(&mut self, update: Update) {
        match update {
            Update::Prefixes(prefixes) => {
                self.running.retain(|id, _| prefixes.contains_key(id));
                self.prefixes = prefixes;
            }
            Update::Groups(prefix, Some(groups)) => {
                self.running.insert(prefix, groups);
            }
            Update::Groups(prefix, None) => {
                self.running.remove(&prefix);
            }
            Update::Addons(addons) => self.addons = addons,
            Update::Catalogs(catalogs) => self.catalogs = catalogs,
            Update::Listing(plugin, listing) => {
                self.listings.insert(plugin, listing);
            }
        }
    }
}

/// Identifies the group subscription of a prefix by the prefix alone.
struct Groups(Prefix);

impl Hash for Groups {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id().hash(state);
    }
}

fn groups(Groups(prefix): &Groups) -> impl Stream<Item = Update> + use<> {
    let id = prefix.id();
    prefix
        .watch_groups()
        .map(move |running| Update::Groups(id, running))
}
