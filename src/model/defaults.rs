//! The addons an installation needs, and the releases a new bottle starts with.
//!
//! Versions are compared only within a series: the releases that share a
//! name. Wine and Proton-GE versions, for example, say nothing about each
//! other.

use bottles_core::{
    Addon, AddonRole, CatalogEntry, Component, Runner, RunnerFlavor, Slot, Umu, WineBridge,
};
use uuid::Uuid;

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
    /// The catalog entry to download for each essential role the catalogs offer.
    pub(crate) fn essential_entries(&self) -> Vec<CatalogEntry> {
        REQUIRED
            .into_iter()
            .chain(OPTIONAL)
            .filter_map(|role| self.essential_entry(role))
            .cloned()
            .collect()
    }

    /// The newest supported entry of the first series the catalogs list for `role`.
    fn essential_entry(&self, role: AddonRole) -> Option<&CatalogEntry> {
        let supported = || {
            self.sources
                .catalogs
                .entries::<AddonRole>()
                .filter(move |entry| entry.role() == role && entry.is_supported())
        };
        let series = supported().next()?.name();
        supported()
            .filter(|entry| entry.name() == series)
            .max_by(|left, right| {
                left.version()
                    .cmp(right.version())
                    .then_with(|| left.id().cmp(&right.id()))
            })
    }

    pub(crate) fn installed(&self, entry: &CatalogEntry) -> bool {
        self.sources
            .addons
            .release::<AddonRole>(entry.id())
            .is_some()
    }

    /// Returns whether a release of every required role is installed.
    pub(crate) fn has_essentials(&self) -> bool {
        REQUIRED.iter().all(|role| {
            self.sources
                .addons
                .releases::<AddonRole>()
                .any(|release| release.role() == *role)
        })
    }

    /// Returns whether a bottle can be created with the installed `runner`.
    ///
    /// Every bottle needs `WineBridge`, and a Proton runner also needs UMU.
    pub(crate) fn can_create_with(&self, runner: Uuid) -> bool {
        let Some(runner) = self.sources.addons.release::<Runner>(runner) else {
            return false;
        };
        self.winebridge().is_some()
            && (runner.flavor() == RunnerFlavor::Wine || self.umu().is_some())
    }

    /// Installed runners, ordered by series and newest first within each.
    pub(crate) fn runners(&self) -> Vec<Addon<Runner>> {
        let mut runners = self.sources.addons.releases::<Runner>().collect::<Vec<_>>();
        runners.sort_by(|left, right| {
            left.name()
                .cmp(right.name())
                .then_with(|| right.version().cmp(left.version()))
                .then_with(|| right.id().cmp(&left.id()))
        });
        runners
    }

    /// The runner a new bottle starts with.
    ///
    /// This is the newest installed release of the essential runner series,
    /// or else the first installed Wine runner.
    pub(crate) fn default_runner(&self) -> Option<Uuid> {
        let runners = self.runners();
        let wine = || {
            runners
                .iter()
                .filter(|runner| runner.flavor() == RunnerFlavor::Wine)
        };
        let series = self.essential_entry(REQUIRED[0]).map(CatalogEntry::name);
        wine()
            .find(|runner| Some(runner.name()) == series)
            .or_else(|| wine().next())
            .map(Addon::id)
    }

    /// The newest installed `WineBridge` release.
    pub(crate) fn winebridge(&self) -> Option<Addon<WineBridge>> {
        newest(self.sources.addons.releases())
    }

    /// The newest installed UMU release.
    pub(crate) fn umu(&self) -> Option<Addon<Umu>> {
        newest(self.sources.addons.releases())
    }
}

/// Returns whether `entries` cover every required role.
pub(crate) fn required_available(entries: &[CatalogEntry]) -> bool {
    REQUIRED
        .iter()
        .all(|role| entries.iter().any(|entry| entry.role() == *role))
}

/// The greatest version among releases of one series, with ties broken by identifier.
fn newest<R>(releases: impl Iterator<Item = Addon<R>>) -> Option<Addon<R>> {
    releases.max_by(|left, right| {
        left.version()
            .cmp(right.version())
            .then_with(|| left.id().cmp(&right.id()))
    })
}
