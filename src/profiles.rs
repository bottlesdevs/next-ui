//! People using this installation and the storefront accounts linked to them.

use std::{io, path::Path};

use next_config::Config;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(version = 1)]
pub(crate) struct Profiles {
    pub(crate) selected: Uuid,
    pub(crate) profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) accounts: Vec<AccountLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccountLink {
    pub(crate) link_id: Uuid,
    pub(crate) provider: Provider,
    pub(crate) identity: Identity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Provider {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Identity {
    pub(crate) account_id: String,
    pub(crate) display_name: String,
}

impl Profiles {
    pub(crate) async fn load(config_dir: &Path) -> Result<Self, String> {
        let path = config_dir.join("profiles.toml");
        match next_config::load(&path).await {
            Ok(profiles) => Ok(profiles),
            Err(next_config::error::Error::Io(error))
                if error.kind() == io::ErrorKind::NotFound =>
            {
                let profile = Profile {
                    id: Uuid::new_v4(),
                    name: "Player".into(),
                    accounts: Vec::new(),
                };
                let profiles = Self {
                    selected: profile.id,
                    profiles: vec![profile],
                };
                next_config::save(&path, &profiles)
                    .await
                    .map_err(|error| format!("failed to save {}: {error}", path.display()))?;
                Ok(profiles)
            }
            Err(error) => Err(format!("failed to load {}: {error}", path.display())),
        }
    }
}
