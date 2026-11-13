//! What the screens read, and the only code that calls core.
//!
//! [`Sources`] mirror core and the plugins, records are the files the
//! application owns, and [`Operations`] are the calls in flight. Queries join
//! the three on every read, and [`Action`]s are the only way to change them.

mod action;
mod collection;
mod components;
mod defaults;
mod library;
mod operations;
mod records;
mod settings;
mod sources;

use std::{future::Future, sync::Arc};

use bottles_core::Core;
use bottles_plugin_host::Plugins;
use iced::{Subscription, Task, futures::future::BoxFuture};
use tokio_util::task::TaskTracker;

pub(crate) use action::{Action, Change};
pub(crate) use components::AvailableRelease;
pub(crate) use defaults::{REQUIRED, required_available};
pub(crate) use library::{Activity, Bottle, Program, Target};
pub(crate) use operations::{Operations, Subject, Verb};
pub(crate) use settings::Experience;
pub(crate) use sources::{Listing, Sources};

use crate::{
    notifications::{Follow, Notification, Notifications},
    profiles::Profiles,
};
use collection::Collection;
use next_ui::widget::InfoCardKind;
use operations::{Finished, Outcome};
use records::Saved;
use settings::Settings;

pub(crate) struct Model {
    core: Arc<Core>,
    plugins: Arc<Plugins>,
    pub(crate) sources: Sources,
    pub(crate) settings: Saved<Settings>,
    collection: Saved<Collection>,
    pub(crate) profiles: Saved<Profiles>,
    pub(crate) operations: Operations,
    pub(crate) notifications: Notifications,
    tracker: TaskTracker,
}

/// A release being downloaded.
pub(crate) struct Download {
    pub(crate) id: uuid::Uuid,
    /// Its name and version, unless the catalogs no longer list it.
    pub(crate) name: Option<String>,
    /// What it's doing, or `None` while it waits to start.
    pub(crate) progress: Option<bottles_core::Progress>,
}

impl Download {
    /// The fraction done, when the size is known.
    pub(crate) fn fraction(&self) -> Option<f32> {
        self.progress
            .as_ref()
            .and_then(bottles_core::Progress::fraction)
    }
}

/// A group of releases on the Addons page, in the order it's listed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseKind {
    Runners,
    Component(bottles_core::Slot),
    /// WineBridge and UMU, which every bottle uses.
    Bridges,
    Dependencies,
}

/// A group of releases on the Addons page.
pub(crate) struct ReleaseGroup {
    pub(crate) kind: ReleaseKind,
    pub(crate) downloaded: Vec<(uuid::Uuid, String)>,
    pub(crate) available: Vec<AvailableRelease>,
}

/// A plugin installed on this computer.
pub(crate) struct InstalledPlugin {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) account: bool,
    pub(crate) library: bool,
}

/// Everything loaded before the model exists.
#[derive(Clone)]
pub(crate) struct Boot {
    core: Arc<Core>,
    plugins: Arc<Plugins>,
    settings: Settings,
    collection: Collection,
    profiles: Profiles,
    /// The installed plugins that failed to load, with why.
    plugin_failures: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub(crate) enum Message {
    Source(sources::Update),
    Operation(Subject, operations::Event),
    Saved(Result<(), String>),
}

impl Model {
    pub(crate) async fn boot() -> Result<Boot, String> {
        let core = Core::builder()
            .build()
            .await
            .map(Arc::new)
            .map_err(|error| error.to_string())?;
        let result = async {
            let plugins = Plugins::new().map_err(|error| error.to_string())?;
            let mut plugin_failures = Vec::new();
            for (id, error) in plugins.reload().await.map_err(|error| error.to_string())? {
                tracing::warn!(plugin = id, %error, "plugin failed to load");
                plugin_failures.push((id, error.to_string()));
            }
            Ok(Boot {
                settings: records::load(&Settings::path()).await?,
                collection: records::load(&Collection::path()).await?,
                profiles: Profiles::load().await?,
                core: core.clone(),
                plugins: Arc::new(plugins),
                plugin_failures,
            })
        }
        .await;
        if result.is_err()
            && let Err(error) = core.shutdown().await
        {
            tracing::error!(%error, "core shutdown failed during boot cleanup");
        }
        result
    }

    pub(crate) fn new(boot: Boot) -> (Self, Task<Message>) {
        let tracker = TaskTracker::new();
        let mut model = Self {
            sources: Sources::new(&boot.core, &boot.plugins),
            settings: Saved::new(boot.settings, Settings::path()),
            collection: Saved::new(boot.collection, Collection::path()),
            profiles: Saved::new(boot.profiles, Profiles::path()),
            operations: Operations::new(tracker.clone()),
            notifications: Notifications::default(),
            core: boot.core,
            plugins: boot.plugins,
            tracker,
        };
        for (id, error) in boot.plugin_failures {
            model.notifications.push(
                Notification::new(
                    InfoCardKind::Warning,
                    "A plugin couldn’t load",
                    format!("{id}: {error}"),
                )
                .action("Open Plugins", Follow::Plugins),
            );
        }
        let watch = Sources::watch(&model.core, &model.plugins).map(Message::Source);
        let prune = model.prune();
        (model, Task::batch([watch, prune]))
    }

    /// Follows the running programs of every recorded prefix.
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let collection = &self.collection;
        let recorded = collection.bottles.keys().chain(collection.programs.keys());
        Sources::follow(&self.core, recorded.copied()).map(Message::Source)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Source(update) => {
                let prefixes = matches!(update, sources::Update::Prefixes(_));
                self.sources.apply(update);
                if prefixes {
                    return self.prune();
                }
            }
            Message::Operation(subject, event) => {
                let Some(Finished { verb, then, result }) = self.operations.update(&subject, event)
                else {
                    return Task::none();
                };
                match (&subject, verb, result) {
                    // Cancelling isn't news: the row the download was in
                    // shows it gone.
                    (_, _, Ok(Outcome::Cancelled)) => {}
                    // Essentials show their progress where they were
                    // started, so finishing needs no notification.
                    (Subject::Addon(release), Verb::Fetch, Ok(outcome)) => {
                        if let Some(then) = then {
                            let name = self.release_name(*release);
                            self.notifications.success(
                                format!("{name} downloaded"),
                                "It’s selected in the bottle.",
                            );
                            return self.follow(then, outcome);
                        }
                    }
                    (Subject::Addon(release), Verb::Fetch, Err(error)) => {
                        let name = self.release_name(*release);
                        let retry = match then {
                            Some(Action::UseRelease { prefix, release }) => {
                                Action::FetchRelease { prefix, release }
                            }
                            _ => Action::Download(*release),
                        };
                        self.notifications.push(
                            Notification::new(
                                InfoCardKind::Error,
                                "Error",
                                format!("Couldn’t download {name}. {error}"),
                            )
                            .action("Retry", Follow::Act(retry)),
                        );
                        return Task::none();
                    }
                    (_, _, Err(error)) => {
                        self.notifications.error(error);
                        return Task::none();
                    }
                    (_, _, Ok(outcome)) => {
                        if let Some(then) = then {
                            return self.follow(then, outcome);
                        }
                    }
                }
            }
            Message::Saved(result) => {
                if let Err(error) = result {
                    self.notifications.error(error);
                }
            }
        }
        Task::none()
    }

    /// Cancels the operations in flight, waits for them and for pending
    /// writes, then shuts core down.
    pub(crate) fn shutdown(&self) -> impl Future<Output = Result<(), String>> + use<> {
        self.operations.shutdown();
        let tracker = self.tracker.clone();
        tracker.close();
        let core = self.core.clone();
        async move {
            tracker.wait().await;
            core.shutdown().await.map_err(|error| error.to_string())
        }
    }

    /// Drops the records whose prefix no longer exists.
    fn prune(&mut self) -> Task<Message> {
        if !self.collection.has_orphans(&self.sources.prefixes) {
            return Task::none();
        }
        let prefixes = self.sources.prefixes.clone();
        self.change_collection(move |collection| collection.prune(&prefixes))
    }

    fn change_collection(&mut self, change: impl FnOnce(&mut Collection)) -> Task<Message> {
        let save = self.collection.change(change);
        self.write(save)
    }

    fn change_profiles(
        &mut self,
        change: impl FnOnce(&mut Profiles) -> Result<(), String>,
    ) -> Task<Message> {
        let mut changed = self.profiles.clone();
        if let Err(error) = change(&mut changed) {
            self.notifications.error(error);
            return Task::none();
        }
        let save = self.profiles.change(move |profiles| *profiles = changed);
        self.write(save)
    }

    /// The storefronts an account can be linked from: built-in Steam
    /// detection and every account plugin, as `(id, name)`.
    pub(crate) fn account_providers(&self) -> Vec<(String, String)> {
        std::iter::once(("steam".to_owned(), "Steam".to_owned()))
            .chain(
                self.plugins
                    .providing::<bottles_plugin_host::Account>()
                    .iter()
                    .map(|plugin| {
                        let manifest = plugin.manifest();
                        (manifest.id.clone(), manifest.name.clone())
                    }),
            )
            .collect()
    }

    /// Releases in the groups Preferences lists them in: runners, then each
    /// component slot, then WineBridge and UMU, then dependencies. Each group
    /// has its downloaded releases, newest first, and the newest ones the
    /// catalogs offer that aren't downloaded; groups with neither are left
    /// out.
    pub(crate) fn release_groups(&self) -> Vec<ReleaseGroup> {
        use bottles_core::{AddonRole, Slot};
        const KINDS: [ReleaseKind; 7] = [
            ReleaseKind::Runners,
            ReleaseKind::Component(Slot::Dxvk),
            ReleaseKind::Component(Slot::Vkd3d),
            ReleaseKind::Component(Slot::Nvapi),
            ReleaseKind::Component(Slot::LatencyFlex),
            ReleaseKind::Bridges,
            ReleaseKind::Dependencies,
        ];
        let kind = |role: AddonRole| match role {
            AddonRole::Runner(_) => ReleaseKind::Runners,
            AddonRole::Component(component) => ReleaseKind::Component(component.slot),
            AddonRole::WineBridge(_) | AddonRole::Umu(_) => ReleaseKind::Bridges,
            AddonRole::Dependency(_) => ReleaseKind::Dependencies,
        };
        KINDS
            .into_iter()
            .map(|group| {
                let mut releases = self
                    .sources
                    .addons
                    .releases::<AddonRole>()
                    .filter(|release| kind(release.role()) == group)
                    .collect::<Vec<_>>();
                releases.sort_by(|left, right| right.version().cmp(left.version()));
                ReleaseGroup {
                    kind: group,
                    downloaded: releases
                        .iter()
                        .map(|release| (release.id(), label(release.name(), release.version())))
                        .collect(),
                    available: self.available(|role| kind(role) == group),
                }
            })
            .filter(|group| !group.downloaded.is_empty() || !group.available.is_empty())
            .collect()
    }

    /// A catalog release's name and version, e.g. "DXVK 2.6", unless the
    /// catalogs no longer list it.
    pub(crate) fn release_label(&self, id: uuid::Uuid) -> Option<String> {
        self.sources
            .catalogs
            .entry::<bottles_core::AddonRole>(id)
            .map(|entry| label(entry.name(), entry.version()))
    }

    /// A catalog release's name and version, or a stand-in.
    fn release_name(&self, id: uuid::Uuid) -> String {
        self.release_label(id)
            .unwrap_or_else(|| "the release".to_owned())
    }

    /// The releases being downloaded, by name.
    pub(crate) fn downloads(&self) -> Vec<Download> {
        let mut downloads: Vec<_> = self
            .operations
            .iter()
            .filter_map(|(subject, pending)| {
                let (Subject::Addon(id), Verb::Fetch) = (subject, &pending.verb) else {
                    return None;
                };
                Some(Download {
                    id: *id,
                    name: self.release_label(*id),
                    progress: pending.progress.clone(),
                })
            })
            .collect();
        downloads.sort_by(|left, right| left.name.cmp(&right.name));
        downloads
    }

    /// Programs to suggest installing when the library is empty, with what
    /// each is.
    ///
    /// A fixed list until the repository of bottle recipes exists to supply
    /// it.
    pub(crate) fn suggestions(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("Steam", "Games and store"),
            ("Battle.net", "Blizzard games"),
            ("Notepad++", "Text editor"),
        ]
    }

    /// The shortcuts a one-off program created in bottle `prefix`, so they
    /// can be added to it as programs.
    #[allow(dead_code)]
    pub(crate) fn new_shortcuts(&self, _prefix: uuid::Uuid) -> Vec<String> {
        todo!("core doesn't report the shortcuts a program creates")
    }

    /// Names the bottles that use release `id`, so Preferences can say
    /// "In use" instead of offering Remove.
    #[allow(dead_code)]
    pub(crate) fn release_users(&self, _id: uuid::Uuid) -> Vec<String> {
        todo!("core doesn't report which prefixes use a release")
    }

    /// The installed plugins, by name, with whether each provides an account
    /// and a library.
    pub(crate) fn installed_plugins(&self) -> Vec<InstalledPlugin> {
        let manifests = match self.plugins.list() {
            Ok(manifests) => manifests,
            Err(error) => {
                tracing::warn!(%error, "failed to list plugins");
                return Vec::new();
            }
        };
        let mut plugins: Vec<_> = manifests
            .into_iter()
            .map(|manifest| InstalledPlugin {
                account: self
                    .plugins
                    .get::<bottles_plugin_host::Account>(&manifest.id)
                    .is_some(),
                library: self
                    .plugins
                    .get::<bottles_plugin_host::Library>(&manifest.id)
                    .is_some(),
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
            })
            .collect();
        plugins.sort_by(|left, right| left.name.cmp(&right.name));
        plugins
    }

    /// What a plugin says it does, and who made it.
    #[allow(dead_code)]
    pub(crate) fn plugin_about(&self, _id: &str) -> (String, String) {
        todo!("plugin manifests have no description or author yet")
    }

    /// The newer version of plugin `id` the catalog offers, if any.
    #[allow(dead_code)]
    pub(crate) fn plugin_update(&self, _id: &str) -> Option<String> {
        todo!("the plugin catalog doesn't exist yet")
    }

    /// The bytes of private data plugin `id` keeps, for its Storage row.
    #[allow(dead_code)]
    pub(crate) fn plugin_storage(&self, _id: &str) -> u64 {
        todo!("the plugin host doesn't report the size of a plugin's data")
    }

    fn write(&self, save: BoxFuture<'static, Result<(), String>>) -> Task<Message> {
        // Register now: shutdown must also wait for writes not yet polled by Iced.
        Task::perform(self.tracker.track_future(save), Message::Saved)
    }
}

/// How a release is named: its name and version, e.g. "DXVK 2.6".
fn label(name: &str, version: &str) -> String {
    format!("{name} {version}")
}
