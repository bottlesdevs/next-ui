use std::sync::Arc;

use bottles_core::{Bottle, BottleState, Bottles, ProfilesConfig};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct BottleSnapshot {
    pub(crate) bottle: Bottle,
    pub(crate) state: Arc<BottleState>,
}

#[derive(Clone)]
pub(crate) struct DomainSnapshots {
    bottles: Vec<BottleSnapshot>,
    profiles: Arc<ProfilesConfig>,
}

impl DomainSnapshots {
    pub(crate) fn new(core: &Bottles) -> Self {
        Self {
            bottles: bottle_snapshots(core.bottles().list()),
            profiles: core.profiles().snapshot(),
        }
    }

    pub(crate) fn bottles(&self) -> &[BottleSnapshot] {
        &self.bottles
    }

    pub(crate) fn bottle(&self, id: Uuid) -> Option<&BottleSnapshot> {
        self.bottles
            .iter()
            .find(|snapshot| snapshot.state.id() == id)
    }

    pub(crate) fn profiles(&self) -> &ProfilesConfig {
        &self.profiles
    }

    pub(crate) fn replace_bottles(&mut self, bottles: Vec<Bottle>) -> bool {
        let next = bottle_snapshots(bottles);
        let changed = self.bottles.len() != next.len()
            || self.bottles.iter().zip(&next).any(|(current, next)| {
                current.bottle.id() != next.bottle.id() || !Arc::ptr_eq(&current.state, &next.state)
            });
        self.bottles = next;
        changed
    }

    pub(crate) fn replace_profiles(&mut self, profiles: Arc<ProfilesConfig>) -> Option<bool> {
        if self.profiles == profiles {
            return None;
        }
        let current = self.profiles.selected();
        let next = profiles.selected();
        let selected_changed = current.id() != next.id() || current.name() != next.name();
        self.profiles = profiles;
        Some(selected_changed)
    }
}

fn bottle_snapshots(bottles: Vec<Bottle>) -> Vec<BottleSnapshot> {
    let mut snapshots = bottles
        .into_iter()
        .filter_map(|bottle| {
            let state = bottle.state().ok()?;
            Some(BottleSnapshot { bottle, state })
        })
        .collect::<Vec<_>>();
    snapshots.sort_unstable_by_key(|snapshot| snapshot.bottle.id());
    snapshots
}
