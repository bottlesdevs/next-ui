use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[cfg(feature = "fvs")]
use bottles_core::ProgramState;
use bottles_core::{AddonsState, BottleState, Bottles};
use bottles_plugin_host::{Library, Plugin};
use iced::Task;

use crate::{app::Message, library::Listing};

pub(crate) struct Store {
    pub(crate) bottles: Vec<Arc<BottleState>>,
    #[cfg(feature = "fvs")]
    pub(crate) programs: Vec<Arc<ProgramState>>,
    pub(crate) plugins: BTreeMap<String, Listing>,
    pub(crate) addons: Arc<AddonsState>,
}

#[derive(Clone, Debug)]
pub(crate) enum Update {
    Bottles(Vec<Arc<BottleState>>),
    #[cfg(feature = "fvs")]
    Programs(Vec<Arc<ProgramState>>),
    Plugin(String, Listing),
    Addons(Arc<AddonsState>),
}

impl Store {
    pub(crate) fn new(core: &Bottles, libraries: &HashMap<String, Arc<Plugin<Library>>>) -> Self {
        Self {
            bottles: Vec::new(),
            #[cfg(feature = "fvs")]
            programs: Vec::new(),
            plugins: libraries
                .keys()
                .map(|id| (id.clone(), Listing::Loading))
                .collect(),
            addons: core.addons().state(),
        }
    }

    pub(crate) fn watch(
        core: &Bottles,
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
            #[cfg(feature = "fvs")]
            Task::run(core.programs().watch(), |programs| {
                Message::Store(Update::Programs(programs))
            }),
            Task::run(core.bottles().watch(), |bottles| {
                Message::Store(Update::Bottles(bottles))
            }),
            Task::run(core.addons().watch(), |addons| {
                Message::Store(Update::Addons(addons))
            }),
        ]);
        Task::batch(listings.chain([watches]))
    }

    pub(crate) fn update(&mut self, update: Update) {
        match update {
            Update::Bottles(mut bottles) => {
                bottles.sort_by(|left, right| {
                    left.name()
                        .cmp(right.name())
                        .then_with(|| left.id().cmp(&right.id()))
                });
                self.bottles = bottles;
            }
            Update::Plugin(id, listing) => {
                self.plugins.insert(id, listing);
            }
            #[cfg(feature = "fvs")]
            Update::Programs(mut programs) => {
                programs.sort_by(|left, right| {
                    left.name()
                        .cmp(right.name())
                        .then_with(|| left.id().cmp(&right.id()))
                });
                self.programs = programs;
            }
            Update::Addons(addons) => self.addons = addons,
        }
    }
}
