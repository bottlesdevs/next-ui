//! Everything the application can be asked to do.

use bottles_core::{
    AddonRole, Operation, Prefix, PrefixBackend, ProgramSpec, Runner, Umu, WineArch, WineBridge,
};
use bottles_plugin_host::Library;
use iced::Task;
use uuid::Uuid;

use super::{
    Message, Model,
    collection::BottleRecord,
    library::Target,
    operations::{Outcome, Subject, Verb},
    settings::Experience,
};

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Launch(Target),
    /// Asks the programs of a prefix to close, then stops it.
    Stop(Uuid),
    /// Ends the processes of a program without asking it to close.
    Kill {
        prefix: Uuid,
        group: Uuid,
    },
    Delete(Uuid),
    CreateBottle {
        name: String,
        runner: Uuid,
    },
    Rename(Uuid, String),
    Edit(Uuid, Change),
    AddProgram,
    FetchEssentials,
    CancelEssentials,
    RefreshCatalog,
    SwitchExperience(Experience),
}

/// A change to how a prefix starts.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Change {
    Gamescope(bool),
    MangoHud(bool),
}

impl Model {
    pub(crate) fn perform(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::Launch(target) => self.launch(target),
            Action::Stop(id) => self.on_prefix(id, Verb::Stop, |prefix| prefix.stop(false)),
            Action::Kill { prefix, group } => match self.core.prefixes().open(prefix) {
                Ok(handle) => {
                    let kill =
                        async move { handle.kill(group).await.map_err(|error| error.to_string()) };
                    self.operations
                        .perform(Subject::Program { prefix, group }, Verb::Stop, kill)
                        .map(|(subject, event)| Message::Operation(subject, event))
                }
                Err(error) => {
                    self.notifications.error(error.to_string());
                    Task::none()
                }
            },
            Action::Delete(id) => {
                let operation = self.core.prefixes().delete(id);
                self.start(Subject::Prefix(id), Verb::Delete, operation)
            }
            Action::CreateBottle { name, runner } => self.create_bottle(name, runner),
            Action::Rename(id, name) => self.change_collection(move |collection| {
                if let Some(bottle) = collection.bottles.get_mut(&id) {
                    bottle.name = name;
                }
            }),
            Action::Edit(id, change) => self.on_prefix(id, Verb::Edit, move |prefix| {
                prefix.edit(move |edit| {
                    match change {
                        Change::Gamescope(enabled) => edit.wrappers().gamescope.enabled = enabled,
                        Change::MangoHud(enabled) => edit.wrappers().mangohud.enabled = enabled,
                    }
                    Ok(())
                })
            }),
            Action::AddProgram => {
                self.notifications
                    .info("Program installation is not available yet");
                Task::none()
            }
            Action::FetchEssentials => {
                let pending = self
                    .essential_entries()
                    .into_iter()
                    .filter(|entry| !self.installed(entry))
                    .collect::<Vec<_>>();
                Task::batch(pending.into_iter().map(|entry| {
                    let operation = self.core.catalogs().fetch::<AddonRole>(entry.id());
                    self.start(Subject::Addon(entry.id()), Verb::Fetch, operation)
                }))
            }
            Action::CancelEssentials => {
                for entry in self.essential_entries() {
                    self.operations.cancel(&Subject::Addon(entry.id()));
                }
                Task::none()
            }
            Action::RefreshCatalog => {
                let operation = self.core.catalogs().refresh();
                self.start(Subject::Catalog, Verb::Refresh, operation)
            }
            Action::SwitchExperience(experience) => {
                let save = self
                    .settings
                    .change(|settings| settings.experience = Some(experience));
                self.write(save)
            }
        }
    }

    fn launch(&mut self, target: Target) -> Task<Message> {
        let subject = target.subject();
        match target {
            Target::Program { prefix, group } => {
                let Some(spec) = self.spec(prefix, group) else {
                    self.notifications.error("This program no longer exists");
                    return Task::none();
                };
                match self.core.prefixes().open(prefix) {
                    Ok(prefix) => self.start(subject, Verb::Launch, prefix.launch(group, spec)),
                    Err(error) => {
                        self.notifications.error(error.to_string());
                        Task::none()
                    }
                }
            }
            Target::Entry { plugin, id } => {
                let Some(library) = self.plugins.get::<Library>(&plugin) else {
                    self.notifications.error("This plugin is no longer loaded");
                    return Task::none();
                };
                let launch = async move {
                    library
                        .launch(&id)
                        .await
                        .map_err(|error| format!("{plugin}: {error}"))
                };
                self.operations
                    .perform(subject, Verb::Launch, launch)
                    .map(|(subject, event)| Message::Operation(subject, event))
            }
        }
    }

    fn create_bottle(&mut self, name: String, runner: Uuid) -> Task<Message> {
        let addons = self.sources.addons.clone();
        let (Some(runner), Some(winebridge)) = (
            addons.release::<Runner>(runner),
            addons.releases::<WineBridge>().next(),
        ) else {
            self.notifications
                .error("A runner and WineBridge are required to create a bottle");
            return Task::none();
        };
        let operation = self.core.prefixes().create(
            PrefixBackend::Standard,
            WineArch::default(),
            runner,
            winebridge,
            addons.releases::<Umu>().next(),
        );
        self.operations
            .start(
                Subject::Creation(Uuid::new_v4()),
                Verb::Create(name),
                operation,
                |prefix| Outcome::Created(prefix.id()),
            )
            .map(|(subject, event)| Message::Operation(subject, event))
    }

    /// Adds the record of a bottle whose prefix was created.
    pub(super) fn record_bottle(&mut self, id: Uuid, name: String) -> Task<Message> {
        self.change_collection(move |collection| {
            collection.bottles.insert(
                id,
                BottleRecord {
                    name,
                    programs: Default::default(),
                },
            );
        })
    }

    /// The launch definition of a bottle's program or of a standalone program.
    fn spec(&self, prefix: Uuid, group: Uuid) -> Option<ProgramSpec> {
        let collection = &self.collection;
        let program = match collection.bottles.get(&prefix) {
            Some(bottle) => bottle.programs.get(&group),
            None => collection.programs.get(&prefix).filter(|_| prefix == group),
        };
        program.map(|program| program.spec.clone())
    }

    fn on_prefix<T: Send + 'static>(
        &mut self,
        id: Uuid,
        verb: Verb,
        operation: impl FnOnce(&Prefix) -> Operation<T>,
    ) -> Task<Message> {
        match self.core.prefixes().open(id) {
            Ok(prefix) => self.start(Subject::Prefix(id), verb, operation(&prefix)),
            Err(error) => {
                self.notifications.error(error.to_string());
                Task::none()
            }
        }
    }

    fn start<T: Send + 'static>(
        &mut self,
        subject: Subject,
        verb: Verb,
        operation: Operation<T>,
    ) -> Task<Message> {
        self.operations
            .start(subject, verb, operation, |_| Outcome::Done)
            .map(|(subject, event)| Message::Operation(subject, event))
    }
}
