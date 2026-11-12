//! What the screens read, and the only code that calls core.
//!
//! [`Sources`] mirror core and the plugins, records are the files the
//! application owns, and [`Operations`] are the calls in flight. Queries join
//! the three on every read, and [`Action`]s are the only way to change them.

mod action;
mod collection;
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
pub(crate) use defaults::{REQUIRED, required_available};
#[cfg(feature = "fvs")]
pub(crate) use library::Program;
pub(crate) use library::{Activity, Bottle, Target};
pub(crate) use operations::{Operations, Subject, Verb};
pub(crate) use settings::Experience;
pub(crate) use sources::{Listing, Sources};

use crate::{notifications::Notifications, profiles::Profiles};
use collection::Collection;
use operations::Outcome;
use records::Saved;
use settings::Settings;

pub(crate) struct Model {
    core: Arc<Core>,
    plugins: Arc<Plugins>,
    pub(crate) sources: Sources,
    pub(crate) settings: Saved<Settings>,
    collection: Saved<Collection>,
    // No screen reads profiles yet.
    #[allow(dead_code)]
    profiles: Profiles,
    pub(crate) operations: Operations,
    pub(crate) notifications: Notifications,
    tracker: TaskTracker,
}

/// Everything loaded before the model exists.
#[derive(Clone)]
pub(crate) struct Boot {
    core: Arc<Core>,
    plugins: Arc<Plugins>,
    settings: Settings,
    collection: Collection,
    profiles: Profiles,
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
            for (id, error) in plugins.reload().await.map_err(|error| error.to_string())? {
                tracing::warn!(plugin = id, %error, "plugin failed to load");
            }
            Ok(Boot {
                settings: records::load(&Settings::path()).await?,
                collection: records::load(&Collection::path()).await?,
                profiles: Profiles::load().await?,
                core: core.clone(),
                plugins: Arc::new(plugins),
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
            profiles: boot.profiles,
            operations: Operations::new(tracker.clone()),
            notifications: Notifications::default(),
            core: boot.core,
            plugins: boot.plugins,
            tracker,
        };
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
            Message::Operation(subject, event) => match self.operations.update(&subject, event) {
                Some((Verb::Create(name), Ok(Outcome::Created(id)))) => {
                    return self.record_bottle(id, name);
                }
                Some((_, Err(error))) => self.notifications.error(error),
                Some((_, Ok(_))) | None => {}
            },
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

    fn write(&self, save: BoxFuture<'static, Result<(), String>>) -> Task<Message> {
        // Register now: shutdown must also wait for writes not yet polled by Iced.
        Task::perform(self.tracker.track_future(save), Message::Saved)
    }
}
