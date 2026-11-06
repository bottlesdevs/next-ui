//! Launchable entries collected from bottles, programs, and library plugins.

use bottles_plugin_host::LibraryEntry;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Entry {
    Bottle(Uuid, Uuid),
    #[cfg(feature = "fvs")]
    Program(Uuid),
    Plugin(String, String),
}

#[derive(Clone, Debug)]
pub(crate) enum Listing {
    Loading,
    Loaded(Vec<LibraryEntry>),
    Failed(String),
}
