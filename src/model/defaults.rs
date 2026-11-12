//! The addons an installation needs before it can create bottles.

use bottles_core::{
    AddonRole, CatalogEntry, Component, Runner, RunnerFlavor, Slot, Umu, WineBridge,
};

use super::Model;

pub(crate) const REQUIRED: [AddonRole; 2] = [
    AddonRole::Runner(Runner {
        flavor: RunnerFlavor::Wine,
    }),
    AddonRole::WineBridge(WineBridge {}),
];
const OPTIONAL: [AddonRole; 4] = [
    AddonRole::Umu(Umu {}),
    AddonRole::Component(Component { slot: Slot::Dxvk }),
    AddonRole::Component(Component { slot: Slot::Vkd3d }),
    AddonRole::Component(Component {
        slot: Slot::LatencyFlex,
    }),
];

impl Model {
    /// One supported catalog entry for each essential role the catalogs offer.
    pub(crate) fn essential_entries(&self) -> Vec<CatalogEntry> {
        REQUIRED
            .into_iter()
            .chain(OPTIONAL)
            .filter_map(|role| {
                self.sources
                    .catalogs
                    .entries::<AddonRole>()
                    .find(|entry| entry.role() == role && entry.is_supported())
                    .cloned()
            })
            .collect()
    }

    pub(crate) fn installed(&self, entry: &CatalogEntry) -> bool {
        self.sources
            .addons
            .release::<AddonRole>(entry.id())
            .is_some()
    }

    /// Returns whether the releases needed to create a bottle are installed.
    pub(crate) fn can_create_bottles(&self) -> bool {
        let addons = &self.sources.addons;
        addons.releases::<Runner>().next().is_some()
            && addons.releases::<WineBridge>().next().is_some()
    }
}

/// Returns whether `entries` cover every required role.
pub(crate) fn required_available(entries: &[CatalogEntry]) -> bool {
    REQUIRED
        .iter()
        .all(|role| entries.iter().any(|entry| entry.role() == *role))
}
