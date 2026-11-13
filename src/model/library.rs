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
    /// The prefix of a standalone program, which is also its process group.
    pub(crate) fn standalone(&self) -> Option<Uuid> {
        match self {
            Self::Program { prefix, group } if prefix == group => Some(*prefix),
            _ => None,
        }
    }

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
    pub(crate) source: Source<'a>,
    pub(crate) target: Target,
}

/// Where a library entry comes from. Entries with one title are listed in
/// this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source<'a> {
    /// A program in the bottle of this name.
    Bottle(&'a str),
    /// A standalone program.
    Standalone,
    /// An entry listed by the plugin with this identifier.
    Plugin(&'a str),
}

/// What is happening to a subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Activity {
    Idle,
    Launching,
    Running,
    Stopping,
    /// Being edited, deleted, created, or downloaded.
    Busy,
}

impl Activity {
    /// Returns whether nothing is in flight: the subject is idle or running.
    pub(crate) fn is_settled(self) -> bool {
        matches!(self, Self::Idle | Self::Running)
    }
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

    /// The standalone program whose prefix and process group are both `id`.
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

    /// Every program and loaded library entry, ordered by title, then source.
    pub(crate) fn entries(&self) -> Vec<Entry<'_>> {
        let mut entries = Vec::new();
        for bottle in self.bottles() {
            entries.extend(bottle.programs().map(|program| Entry {
                title: program.name,
                source: Source::Bottle(bottle.name),
                target: program.target(),
            }));
        }
        let standalone = self.collection.programs.keys();
        entries.extend(
            standalone
                .filter_map(|id| self.program(*id))
                .map(|program| Entry {
                    title: program.name,
                    source: Source::Standalone,
                    target: program.target(),
                }),
        );
        for (plugin, listing) in &self.sources.listings {
            if let Listing::Loaded(listed) = listing {
                entries.extend(listed.iter().map(|entry| Entry {
                    title: &entry.title,
                    source: Source::Plugin(plugin),
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
    /// An operation in flight comes first, and a program is also affected by
    /// the operation on its prefix. Otherwise a prefix is running while Wine
    /// runs in it, and a program while its process group has a process.
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
            None => {
                let running = &self.sources.running;
                let running = match subject {
                    Subject::Prefix(prefix) => running.contains_key(prefix),
                    Subject::Program { prefix, group } => running
                        .get(prefix)
                        .is_some_and(|groups| groups.contains(group)),
                    _ => false,
                };
                if running {
                    Activity::Running
                } else {
                    Activity::Idle
                }
            }
            Some(Verb::Launch) => Activity::Launching,
            Some(Verb::Stop) => Activity::Stopping,
            Some(_) => Activity::Busy,
        }
    }
}
