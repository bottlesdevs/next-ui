use std::path::PathBuf;

use bottles_core::PrefixBackend;
use next_config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) experience: Option<Experience>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Experience {
    Next,
    Classic,
}

impl Experience {
    /// Returns whether this build can show the experience.
    ///
    /// Next gives every program a prefix of its own, which needs Virgo.
    pub(crate) fn is_available(self) -> bool {
        match self {
            Self::Classic => true,
            Self::Next => PrefixBackend::Virgo.is_available(),
        }
    }
}

impl Settings {
    pub(crate) fn path() -> PathBuf {
        next_paths::config_dir().join("ui.toml")
    }
}
