//! Bottles and standalone programs, kept by the UI over core prefixes.
//!
//! Each record is keyed by the identifier of the prefix it describes.

use std::{collections::BTreeMap, path::PathBuf};

use bottles_core::{Inventory, ProgramSpec};
use next_config::Config;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
pub(crate) struct Collection {
    #[serde(default)]
    pub(crate) bottles: BTreeMap<Uuid, BottleRecord>,
    #[serde(default)]
    pub(crate) programs: BTreeMap<Uuid, ProgramRecord>,
}

/// A named prefix holding any number of programs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BottleRecord {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) programs: BTreeMap<Uuid, ProgramRecord>,
}

/// A named launch definition. Its identifier is also its process group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProgramRecord {
    pub(crate) name: String,
    pub(crate) spec: ProgramSpec,
}

impl Collection {
    pub(crate) fn path() -> PathBuf {
        next_paths::data_dir().join("collection.toml")
    }

    /// Returns whether a record describes a prefix that is not in `prefixes`.
    pub(crate) fn has_orphans(&self, prefixes: &Inventory) -> bool {
        self.bottles
            .keys()
            .chain(self.programs.keys())
            .any(|id| !prefixes.contains_key(id))
    }

    /// Drops the records whose prefix is not in `prefixes`.
    pub(crate) fn prune(&mut self, prefixes: &Inventory) {
        self.bottles.retain(|id, _| prefixes.contains_key(id));
        self.programs.retain(|id, _| prefixes.contains_key(id));
    }
}
