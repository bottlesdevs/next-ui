use std::{future::Future, io, path::Path};

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
    pub(crate) async fn load(config_dir: &Path) -> Result<Self, String> {
        let path = config_dir.join("ui.toml");
        match next_config::load(&path).await {
            Ok(config) => Ok(config),
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
        config_dir: &Path,
    ) -> impl Future<Output = Result<(), String>> + Send + 'static {
        let path = config_dir.join("ui.toml");
        async move {
            next_config::save(&path, &self)
                .await
                .map_err(|error| format!("failed to save {}: {error}", path.display()))
        }
    }
}
