use std::sync::Arc;

#[cfg(feature = "fvs")]
use bottles_core::ProgramState;
use bottles_core::{AddonsState, BottleState, Bottles, LibrarySnapshot};
use iced::Task;

use crate::app::Message;

pub(crate) struct Store {
    pub(crate) bottles: Vec<Arc<BottleState>>,
    #[cfg(feature = "fvs")]
    pub(crate) programs: Vec<Arc<ProgramState>>,
    pub(crate) library: LibrarySnapshot,
    pub(crate) addons: Arc<AddonsState>,
}

#[derive(Clone, Debug)]
pub(crate) enum Update {
    Bottles(Vec<Arc<BottleState>>),
    #[cfg(feature = "fvs")]
    Programs(Vec<Arc<ProgramState>>),
    Library(LibrarySnapshot),
    Addons(Arc<AddonsState>),
}

impl Store {
    pub(crate) fn new(core: &Bottles) -> Self {
        Self {
            bottles: Vec::new(),
            #[cfg(feature = "fvs")]
            programs: Vec::new(),
            library: LibrarySnapshot::default(),
            addons: core.addons().state(),
        }
    }

    pub(crate) fn watch(core: &Bottles) -> Task<Message> {
        Task::batch([
            #[cfg(feature = "fvs")]
            Task::run(core.programs().watch(), |programs| {
                Message::Store(Update::Programs(programs))
            }),
            Task::run(core.bottles().watch(), |bottles| {
                Message::Store(Update::Bottles(bottles))
            }),
            Task::run(core.library().watch(), |library| {
                Message::Store(Update::Library(library))
            }),
            Task::run(core.addons().watch(), |addons| {
                Message::Store(Update::Addons(addons))
            }),
        ])
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
            Update::Library(library) => self.library = library,
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
