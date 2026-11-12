//! Bottles, programs, and launchable entries, joined from records and sources.

use std::{collections::BTreeMap, sync::Arc};

use bottles_core::{PrefixState, ProgramSpec};
use uuid::Uuid;

use super::{
    Model,
    collection::ProgramRecord,
    operations::{Subject, Verb},
    sources::Listing,
};

/// Something that can be launched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// A program, launched in `prefix` as process group `group`.
    Program { prefix: Uuid, group: Uuid },
    /// A library entry, launched by its plugin.
    Entry { plugin: String, id: String },
}

impl Target {
    pub(crate) fn subject(&self) -> Subject {
        match self.clone() {
            Self::Program { prefix, group } => Subject::Program { prefix, group },
            Self::Entry { plugin, id } => Subject::Entry { plugin, id },
        }
    }
}

/// A bottle record joined with the state of its prefix.
#[derive(Clone, Copy)]
pub(crate) struct Bottle<'a> {
    pub(crate) id: Uuid,
    pub(crate) name: &'a str,
    pub(crate) state: &'a Arc<PrefixState>,
    programs: &'a BTreeMap<Uuid, ProgramRecord>,
}

impl<'a> Bottle<'a> {
    pub(crate) fn programs(self) -> impl Iterator<Item = Program<'a>> {
        self.programs.iter().map(move |(group, record)| Program {
            prefix: self.id,
            group: *group,
            name: &record.name,
            spec: &record.spec,
            state: self.state,
        })
    }
}

/// A program record joined with the state of the prefix it runs in.
// Only the Next experience reads the launch definition and prefix state.
#[cfg_attr(not(feature = "fvs"), allow(dead_code))]
#[derive(Clone, Copy)]
pub(crate) struct Program<'a> {
    pub(crate) prefix: Uuid,
    pub(crate) group: Uuid,
    pub(crate) name: &'a str,
    pub(crate) spec: &'a ProgramSpec,
    pub(crate) state: &'a Arc<PrefixState>,
}

impl Program<'_> {
    pub(crate) fn target(&self) -> Target {
        Target::Program {
            prefix: self.prefix,
            group: self.group,
        }
    }
}

/// One title of the library and where it comes from.
pub(crate) struct Entry<'a> {
    pub(crate) title: &'a str,
    pub(crate) source: &'a str,
    pub(crate) target: Target,
}

/// What is happening to a subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Activity {
    Idle,
    Launching,
    Stopping,
    /// Being edited, deleted, created, or downloaded.
    Busy,
}

impl Model {
    /// Bottles whose prefix exists, ordered by name.
    pub(crate) fn bottles(&self) -> Vec<Bottle<'_>> {
        let mut bottles = self
            .collection
            .bottles
            .iter()
            .filter_map(|(id, record)| {
                Some(Bottle {
                    id: *id,
                    name: &record.name,
                    state: self.sources.prefixes.get(id)?,
                    programs: &record.programs,
                })
            })
            .collect::<Vec<_>>();
        bottles.sort_by_key(|bottle| bottle.name);
        bottles
    }

    pub(crate) fn bottle(&self, id: Uuid) -> Option<Bottle<'_>> {
        let record = self.collection.bottles.get(&id)?;
        Some(Bottle {
            id,
            name: &record.name,
            state: self.sources.prefixes.get(&id)?,
            programs: &record.programs,
        })
    }

    /// Standalone programs whose prefix exists, ordered by name.
    #[cfg(feature = "fvs")]
    pub(crate) fn programs(&self) -> Vec<Program<'_>> {
        let mut programs = self
            .collection
            .programs
            .keys()
            .filter_map(|id| self.program(*id))
            .collect::<Vec<_>>();
        programs.sort_by_key(|program| program.name);
        programs
    }

    /// The standalone program whose prefix and process group are both `id`.
    #[cfg(feature = "fvs")]
    pub(crate) fn program(&self, id: Uuid) -> Option<Program<'_>> {
        let record = self.collection.programs.get(&id)?;
        Some(Program {
            prefix: id,
            group: id,
            name: &record.name,
            spec: &record.spec,
            state: self.sources.prefixes.get(&id)?,
        })
    }

    /// Every program and loaded library entry, ordered by title and source.
    pub(crate) fn entries(&self) -> Vec<Entry<'_>> {
        let mut entries = Vec::new();
        for bottle in self.bottles() {
            entries.extend(bottle.programs().map(|program| Entry {
                title: program.name,
                source: bottle.name,
                target: program.target(),
            }));
        }
        #[cfg(feature = "fvs")]
        entries.extend(self.programs().into_iter().map(|program| Entry {
            title: program.name,
            source: "Program",
            target: program.target(),
        }));
        for (plugin, listing) in &self.sources.listings {
            if let Listing::Loaded(listed) = listing {
                entries.extend(listed.iter().map(|entry| Entry {
                    title: &entry.title,
                    source: plugin,
                    target: Target::Entry {
                        plugin: plugin.clone(),
                        id: entry.id.clone(),
                    },
                }));
            }
        }
        entries.sort_by(|left, right| (left.title, left.source).cmp(&(right.title, right.source)));
        entries
    }

    /// Returns what is happening to `subject`.
    ///
    /// A program is also affected by what happens to its prefix.
    pub(crate) fn activity(&self, subject: &Subject) -> Activity {
        let prefix = match subject {
            Subject::Program { prefix, .. } => Some(Subject::Prefix(*prefix)),
            _ => None,
        };
        let pending = self
            .operations
            .get(subject)
            .or_else(|| self.operations.get(prefix.as_ref()?));
        match pending.map(|pending| &pending.verb) {
            None => Activity::Idle,
            Some(Verb::Launch) => Activity::Launching,
            Some(Verb::Stop) => Activity::Stopping,
            Some(_) => Activity::Busy,
        }
    }
}
