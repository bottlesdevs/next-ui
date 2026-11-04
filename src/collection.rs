//! Bottles and standalone programs, kept by the UI over core environments.
//!
//! Each record is keyed by the identifier of the environment it describes.

use std::{collections::BTreeMap, future::Future, io, path::Path, sync::Arc};

use bottles_core::{EnvironmentState, ProgramSpec};
use next_config::Config;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::Store;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
pub(crate) struct Collection {
    #[serde(default)]
    pub(crate) bottles: BTreeMap<Uuid, Bottle>,
    #[serde(default)]
    pub(crate) programs: BTreeMap<Uuid, Program>,
}

/// A named environment holding any number of programs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Bottle {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) programs: BTreeMap<Uuid, Program>,
}

/// A named launch definition. Its identifier is also its process group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Program {
    pub(crate) name: String,
    pub(crate) spec: ProgramSpec,
}

/// A change applied once the environment work behind it has finished.
#[derive(Debug, Clone)]
pub(crate) enum Change {
    AddBottle(Uuid, String),
    Remove(Uuid),
}

/// A record joined with the current state of its environment.
pub(crate) struct Item<'a, R> {
    pub(crate) id: Uuid,
    pub(crate) record: &'a R,
    pub(crate) state: &'a Arc<EnvironmentState>,
}

impl<R> Clone for Item<'_, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for Item<'_, R> {}

impl Collection {
    pub(crate) async fn load(data_dir: &Path) -> Result<Self, String> {
        let path = Self::path(data_dir);
        match next_config::load(&path).await {
            Ok(collection) => Ok(collection),
            Err(next_config::error::Error::Io(error))
                if error.kind() == io::ErrorKind::NotFound =>
            {
                Ok(Self::default())
            }
            Err(error) => Err(format!("failed to load {}: {error}", path.display())),
        }
    }

    pub(crate) fn save(
        self,
        data_dir: &Path,
    ) -> impl Future<Output = Result<(), String>> + Send + 'static {
        let path = Self::path(data_dir);
        async move {
            next_config::save(&path, &self)
                .await
                .map_err(|error| format!("failed to save {}: {error}", path.display()))
        }
    }

    pub(crate) fn apply(&mut self, change: Change) {
        match change {
            Change::AddBottle(id, name) => {
                self.bottles.insert(
                    id,
                    Bottle {
                        name,
                        programs: BTreeMap::new(),
                    },
                );
            }
            Change::Remove(id) => {
                self.bottles.remove(&id);
                self.programs.remove(&id);
            }
        }
    }

    fn path(data_dir: &Path) -> std::path::PathBuf {
        data_dir.join("collection.toml")
    }

    /// Bottles whose environment exists, ordered by name.
    pub(crate) fn bottles<'a>(&'a self, store: &'a Store) -> Vec<Item<'a, Bottle>> {
        join(&self.bottles, store, |bottle| &bottle.name)
    }

    /// Standalone programs whose environment exists, ordered by name.
    #[cfg(feature = "fvs")]
    pub(crate) fn programs<'a>(&'a self, store: &'a Store) -> Vec<Item<'a, Program>> {
        join(&self.programs, store, |program| &program.name)
    }
}

fn join<'a, R>(
    records: &'a BTreeMap<Uuid, R>,
    store: &'a Store,
    name: impl Fn(&R) -> &String,
) -> Vec<Item<'a, R>> {
    let mut items = records
        .iter()
        .filter_map(|(id, record)| {
            Some(Item {
                id: *id,
                record,
                state: store.environments.get(id)?,
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| name(left.record).cmp(name(right.record)));
    items
}
