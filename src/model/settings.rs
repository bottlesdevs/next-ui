use std::path::PathBuf;

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
    pub(crate) fn is_available(self) -> bool {
        self == Self::Classic || cfg!(feature = "fvs")
    }
}

impl Settings {
    pub(crate) fn path() -> PathBuf {
        next_paths::config_dir().join("ui.toml")
    }
}
