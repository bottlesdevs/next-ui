//! People using this installation and the storefront accounts linked to them.
//!
//! Accounts are linked through the built-in Steam detection or an account
//! plugin. Only public identities are kept here; a plugin keeps its own session.

// No screen manages profiles or accounts yet.
#![allow(dead_code)]

mod steam;

use std::{io, path::PathBuf, sync::Arc};

use bottles_plugin_host::{Account, Interaction, Plugin};
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
    pub(crate) async fn load() -> Result<Self, String> {
        let path = Self::path();
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
                profiles.save().await?;
                Ok(profiles)
            }
            Err(error) => Err(format!("failed to load {}: {error}", path.display())),
        }
    }

    pub(crate) async fn save(&self) -> Result<(), String> {
        let path = Self::path();
        next_config::save(&path, self)
            .await
            .map_err(|error| format!("failed to save {}: {error}", path.display()))
    }

    fn path() -> PathBuf {
        next_paths::config_dir().join("profiles.toml")
    }

    /// Adds a profile named `name` and selects it.
    pub(crate) fn create(&mut self, name: &str) -> Uuid {
        let profile = Profile {
            id: Uuid::new_v4(),
            name: name.trim().to_owned(),
            accounts: Vec::new(),
        };
        self.selected = profile.id;
        self.profiles.push(profile);
        self.selected
    }

    pub(crate) fn rename(&mut self, id: Uuid, name: &str) -> Result<(), String> {
        self.profile_mut(id)?.name = name.trim().to_owned();
        Ok(())
    }

    pub(crate) fn select(&mut self, id: Uuid) -> Result<(), String> {
        self.profile_mut(id)?;
        self.selected = id;
        Ok(())
    }

    /// Removes a profile and its links; the last profile cannot be removed.
    pub(crate) fn delete(&mut self, id: Uuid) -> Result<(), String> {
        self.profile_mut(id)?;
        if self.profiles.len() == 1 {
            return Err("the last profile cannot be deleted".into());
        }
        self.profiles.retain(|profile| profile.id != id);
        if self.selected == id {
            self.selected = self.profiles[0].id;
        }
        Ok(())
    }

    /// Adds `identity` from `provider` to a profile, one link per provider.
    pub(crate) fn link(
        &mut self,
        id: Uuid,
        provider: Provider,
        identity: Identity,
    ) -> Result<Uuid, String> {
        let profile = self.profile_mut(id)?;
        if profile
            .accounts
            .iter()
            .any(|account| account.provider.id == provider.id)
        {
            return Err(format!("profile already has a {} account", provider.name));
        }
        let link_id = Uuid::new_v4();
        profile.accounts.push(AccountLink {
            link_id,
            provider,
            identity,
        });
        Ok(link_id)
    }

    pub(crate) fn unlink(&mut self, link_id: Uuid) {
        for profile in &mut self.profiles {
            profile
                .accounts
                .retain(|account| account.link_id != link_id);
        }
    }

    fn profile_mut(&mut self, id: Uuid) -> Result<&mut Profile, String> {
        self.profiles
            .iter_mut()
            .find(|profile| profile.id == id)
            .ok_or_else(|| format!("profile {id} was not found"))
    }
}

/// Identifies the most recently used local Steam account.
pub(crate) async fn steam_account() -> Result<(Provider, Identity), String> {
    let identity = steam::active_account()
        .await
        .map_err(|error| error.to_string())?
        .ok_or("Steam has no active local account")?;
    Ok((
        Provider {
            id: "steam".into(),
            name: "Steam".into(),
        },
        identity,
    ))
}

/// Runs the account plugin's link flow, answering its prompts with `interaction`.
pub(crate) async fn plugin_account(
    provider: Provider,
    plugin: &Plugin<Account>,
    interaction: Arc<dyn Interaction>,
) -> Result<(Provider, Identity), String> {
    let identity = plugin
        .link_account(interaction)
        .await
        .map_err(|error| error.to_string())?;
    Ok((
        provider,
        Identity {
            account_id: identity.account_id,
            display_name: identity.display_name,
        },
    ))
}
