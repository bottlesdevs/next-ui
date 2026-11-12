//! The component releases a bottle can use in each slot.

use bottles_core::{AddonRole, Component, Slot, compare_versions};
use uuid::Uuid;

use super::{
    Model,
    operations::{Subject, Verb},
};

/// How many of the newest releases not yet downloaded a picker offers.
const AVAILABLE_LIMIT: usize = 5;

/// A release of a slot that isn't downloaded yet.
pub(crate) struct AvailableRelease {
    pub(crate) id: Uuid,
    pub(crate) label: String,
    pub(crate) download: Option<ReleaseDownload>,
}

/// A release being downloaded.
pub(crate) struct ReleaseDownload {
    /// The fraction done, when the size is known.
    pub(crate) progress: Option<f32>,
    /// The bottle the release is selected in once it's downloaded.
    pub(crate) then_select: Option<Uuid>,
}

impl Model {
    /// The downloaded releases for `slot`, newest first, with their labels.
    pub(crate) fn component_releases(&self, slot: Slot) -> Vec<(Uuid, String)> {
        let mut releases = self
            .sources
            .addons
            .releases::<Component>()
            .filter(|release| release.slot() == slot)
            .collect::<Vec<_>>();
        releases.sort_by(|left, right| compare_versions(right.version(), left.version()));
        releases
            .into_iter()
            .map(|release| {
                (
                    release.id(),
                    format!("{} {}", release.name(), release.version()),
                )
            })
            .collect()
    }

    /// The newest supported releases for `slot` the catalogs offer that
    /// aren't downloaded, with any download in flight.
    pub(crate) fn available_components(&self, slot: Slot) -> Vec<AvailableRelease> {
        let role = AddonRole::Component(Component { slot });
        let mut entries = self
            .sources
            .catalogs
            .entries::<AddonRole>()
            .filter(|entry| entry.role() == role && entry.is_supported() && !self.installed(entry))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| compare_versions(right.version(), left.version()));
        entries
            .into_iter()
            .take(AVAILABLE_LIMIT)
            .map(|entry| AvailableRelease {
                id: entry.id(),
                label: format!("{} {}", entry.name(), entry.version()),
                download: self
                    .operations
                    .get(&Subject::Addon(entry.id()))
                    .and_then(|pending| match pending.verb {
                        Verb::Fetch { then_select } => Some(ReleaseDownload {
                            progress: pending
                                .progress
                                .as_ref()
                                .and_then(bottles_core::Progress::fraction),
                            then_select,
                        }),
                        _ => None,
                    }),
            })
            .collect()
    }
}
