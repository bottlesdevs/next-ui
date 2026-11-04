//! Steam's most recently used local account, read from `loginusers.vdf`.
//!
//! Detection never changes Steam's active user.

use std::{io, path::PathBuf};

use super::Identity;

#[cfg(target_os = "macos")]
const LOGINUSERS_PATHS: &[&str] = &["Library/Application Support/Steam/config/loginusers.vdf"];

#[cfg(target_os = "linux")]
const LOGINUSERS_PATHS: &[&str] = &[
    ".steam/steam/config/loginusers.vdf",
    ".local/share/Steam/config/loginusers.vdf",
    ".var/app/com.valvesoftware.Steam/.local/share/Steam/config/loginusers.vdf",
];

/// Returns the account marked `MostRecent`, or `None` when Steam is absent.
///
/// The VDF account key is the account ID. A non-empty `AccountName` is the
/// display name; otherwise the account ID is used for both.
pub(super) async fn active_account() -> io::Result<Option<Identity>> {
    let Some(path) = loginusers_path() else {
        return Ok(None);
    };
    parse_active_account(&async_fs::read_to_string(path).await?)
}

fn loginusers_path() -> Option<PathBuf> {
    let home = directories::BaseDirs::new()?.home_dir().to_owned();
    LOGINUSERS_PATHS
        .iter()
        .map(|path| home.join(path))
        .find(|path| path.exists())
}

fn parse_active_account(text: &str) -> io::Result<Option<Identity>> {
    let vdf = keyvalues_parser::parse(text)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let users = vdf.value.get_obj().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "loginusers.vdf root is not an object",
        )
    })?;
    for (steam_id, values) in users.iter() {
        let Some(account) = values.first().and_then(|value| value.get_obj()) else {
            continue;
        };
        if field(account, "MostRecent") != Some("1") {
            continue;
        }
        let account_id = steam_id.to_string();
        let display_name = field(account, "AccountName")
            .filter(|name| !name.is_empty())
            .unwrap_or(&account_id)
            .to_owned();
        return Ok(Some(Identity {
            account_id,
            display_name,
        }));
    }
    Ok(None)
}

fn field<'a>(object: &'a keyvalues_parser::Obj<'_>, name: &str) -> Option<&'a str> {
    object.get(name)?.first()?.get_str()
}
