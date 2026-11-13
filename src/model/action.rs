//! Everything the application can be asked to do.

use bottles_core::{
    AddonRole, Component, Operation, Prefix, ProgramSpec, Runner, RunnerFlavor, Slot, WineArch,
};
use bottles_plugin_host::Library;
use iced::Task;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::{
    Message, Model,
    collection::{BottleRecord, ProgramRecord},
    defaults,
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
        arch: WineArch,
    },
    Rename(Uuid, String),
    /// Selects downloaded `release`, a runner or a component, in bottle
    /// `prefix`.
    UseRelease {
        prefix: Uuid,
        release: Uuid,
    },
    /// Empties `slot` in bottle `prefix`.
    RemoveComponent {
        prefix: Uuid,
        slot: Slot,
    },
    /// Downloads `release` without selecting it anywhere.
    Download(Uuid),
    /// Downloads `release`, then selects it in bottle `prefix`.
    FetchRelease {
        prefix: Uuid,
        release: Uuid,
    },
    Edit(Uuid, Change),
    /// Adds `executable`, a host path, as a program with a prefix of its own.
    AddStandalone {
        name: String,
        executable: PathBuf,
        options: String,
    },
    /// Adds `executable`, a host path, as a program of bottle `prefix`.
    AddToBottle {
        prefix: Uuid,
        name: String,
        executable: PathBuf,
        options: String,
    },
    /// Installs a suggested program from its bottle recipe.
    InstallSuggestion(&'static str),
    /// Asks for an executable, then runs it once in a bottle.
    RunProgram(Uuid),
    /// Runs `executable`, a host path, once in bottle `prefix`.
    Run {
        prefix: Uuid,
        executable: PathBuf,
    },
    FetchEssentials,
    CancelEssentials,
    RefreshCatalog,
    SwitchExperience(Experience),
    SelectProfile(Uuid),
    CreateProfile,
    RenameProfile(Uuid, String),
    DeleteProfile(Uuid),
    Unlink(Uuid),
    /// Removes a downloaded release from shared storage.
    RemoveRelease(Uuid),
    UninstallPlugin(String),
    /// Stops downloading a release.
    CancelDownload(Uuid),
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
            Action::SelectProfile(id) => self.change_profiles(|profiles| profiles.select(id)),
            Action::CreateProfile => self.change_profiles(|profiles| {
                profiles.create("New Profile");
                Ok(())
            }),
            Action::RenameProfile(id, name) => {
                self.change_profiles(move |profiles| profiles.rename(id, &name))
            }
            Action::DeleteProfile(id) => self.change_profiles(|profiles| profiles.delete(id)),
            Action::RemoveRelease(id) => {
                let core = self.core.clone();
                let remove = async move {
                    core.addons()
                        .remove(id)
                        .await
                        .map_err(|error| error.to_string())
                };
                self.operations
                    .perform(Subject::Addon(id), Verb::Delete, remove)
                    .map(|(subject, event)| Message::Operation(subject, event))
            }
            Action::UninstallPlugin(id) => {
                let plugins = self.plugins.clone();
                let uninstall = async move {
                    plugins.uninstall(&id).map_err(|error| error.to_string())?;
                    for (id, error) in plugins.reload().await.map_err(|error| error.to_string())? {
                        tracing::warn!(plugin = id, %error, "plugin failed to load");
                    }
                    Ok(())
                };
                Task::perform(uninstall, |result: Result<(), String>| {
                    Message::Saved(result)
                })
            }
            Action::CancelDownload(id) => {
                self.operations.cancel(&Subject::Addon(id));
                Task::none()
            }
            Action::Unlink(link) => self.change_profiles(|profiles| {
                profiles.unlink(link);
                Ok(())
            }),
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
            Action::CreateBottle { name, runner, arch } => {
                self.create_prefix(name, runner, arch, None)
            }
            Action::Rename(id, name) => self.change_collection(move |collection| {
                if let Some(bottle) = collection.bottles.get_mut(&id) {
                    bottle.name = name;
                }
            }),
            Action::UseRelease { prefix, release } => {
                // Read from core rather than the published snapshot, which
                // may not have caught up with a download that just finished.
                let releases = self.core.addons().state();
                if let Some(component) = releases.release::<Component>(release) {
                    return self.on_prefix(prefix, Verb::Edit, move |handle| {
                        handle.edit(move |edit| {
                            edit.set_component(component);
                            Ok(())
                        })
                    });
                }
                let Some(runner) = releases.release::<Runner>(release) else {
                    self.notifications
                        .error("This release is no longer downloaded");
                    return Task::none();
                };
                // Only Proton runners are launched through UMU, as at creation.
                let umu = match runner.flavor() {
                    RunnerFlavor::Proton => match self.umu() {
                        Some(umu) => Some(umu),
                        None => {
                            self.notifications
                                .error("Proton runners need UMU, which isn't downloaded");
                            return Task::none();
                        }
                    },
                    RunnerFlavor::Wine => None,
                };
                self.on_prefix(prefix, Verb::Edit, move |handle| {
                    handle.edit(move |edit| {
                        edit.set_runner(runner);
                        edit.set_umu(umu);
                        Ok(())
                    })
                })
            }
            Action::RemoveComponent { prefix, slot } => {
                let occupied = self
                    .bottle(prefix)
                    .is_some_and(|bottle| bottle.state.component(slot).is_some());
                if !occupied {
                    return Task::none();
                }
                self.on_prefix(prefix, Verb::Edit, move |handle| {
                    handle.edit(move |edit| edit.remove_component(slot))
                })
            }
            Action::Download(release) => {
                let operation = self.core.catalogs().fetch::<AddonRole>(release);
                self.start(
                    Subject::Addon(release),
                    Verb::Fetch { then_select: None },
                    operation,
                )
            }
            Action::FetchRelease { prefix, release } => {
                let operation = self.core.catalogs().fetch::<AddonRole>(release);
                self.start(
                    Subject::Addon(release),
                    Verb::Fetch {
                        then_select: Some(prefix),
                    },
                    operation,
                )
            }
            Action::Edit(id, change) => self.on_prefix(id, Verb::Edit, move |prefix| {
                prefix.edit(move |edit| {
                    match change {
                        Change::Gamescope(enabled) => edit.wrappers().gamescope.enabled = enabled,
                        Change::MangoHud(enabled) => edit.wrappers().mangohud.enabled = enabled,
                    }
                    Ok(())
                })
            }),
            Action::RunProgram(prefix) => Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Run a Program")
                    .add_filter("Windows programs", &["exe"])
                    .pick_file(),
                move |file| {
                    Message::Chosen(file.map(|file| Action::Run {
                        prefix,
                        executable: file.path().to_owned(),
                    }))
                },
            ),
            Action::Run { prefix, executable } => match self.core.prefixes().open(prefix) {
                // A one-off run gets its own process group and no record;
                // the shortcuts it creates are what become programs
                // (`Model::new_shortcuts`).
                Ok(handle) => {
                    let group = Uuid::new_v4();
                    let spec = ProgramSpec::new(wine_path(&executable));
                    self.start(
                        Subject::Program { prefix, group },
                        Verb::Launch,
                        handle.launch(group, spec),
                    )
                }
                Err(error) => {
                    self.notifications.error(error.to_string());
                    Task::none()
                }
            },
            Action::InstallSuggestion(name) => {
                self.notifications.info(format!(
                    "Installing {name} needs bottle recipes, which aren’t available yet"
                ));
                Task::none()
            }
            Action::AddStandalone {
                name,
                executable,
                options,
            } => {
                let Some(runner) = self.default_runner() else {
                    self.notifications
                        .error("A runner is required to add a program");
                    return Task::none();
                };
                let spec = program_spec(&executable, &options);
                self.create_prefix(name, runner, WineArch::Win64, Some(spec))
            }
            Action::AddToBottle {
                prefix,
                name,
                executable,
                options,
            } => {
                let program = ProgramRecord {
                    name,
                    spec: program_spec(&executable, &options),
                };
                self.change_collection(move |collection| {
                    if let Some(bottle) = collection.bottles.get_mut(&prefix) {
                        bottle.programs.insert(Uuid::new_v4(), program);
                    }
                })
            }
            Action::FetchEssentials => {
                let pending = self
                    .essential_entries()
                    .into_iter()
                    .filter(|entry| !self.installed(entry))
                    .collect::<Vec<_>>();
                Task::batch(pending.into_iter().map(|entry| {
                    let operation = self.core.catalogs().fetch::<AddonRole>(entry.id());
                    self.start(
                        Subject::Addon(entry.id()),
                        Verb::Fetch { then_select: None },
                        operation,
                    )
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

    /// Creates a prefix for a bottle named `name`, or with `program`, for a
    /// standalone program.
    fn create_prefix(
        &mut self,
        name: String,
        runner: Uuid,
        arch: WineArch,
        program: Option<ProgramSpec>,
    ) -> Task<Message> {
        let (Some(runner), Some(winebridge)) = (
            self.sources.addons.release::<Runner>(runner),
            self.winebridge(),
        ) else {
            self.notifications
                .error("A runner and WineBridge are required to create a bottle");
            return Task::none();
        };
        // Only Proton runners are launched through UMU.
        let umu = match runner.flavor() {
            RunnerFlavor::Proton => self.umu(),
            RunnerFlavor::Wine => None,
        };
        let operation =
            self.core
                .prefixes()
                .create(defaults::BACKEND, arch, runner, winebridge, umu);
        self.operations
            .start(
                Subject::Creation(Uuid::new_v4()),
                Verb::Create { name, program },
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

    /// Adds the record of a standalone program whose prefix was created. Its
    /// process group is its prefix.
    pub(super) fn record_program(
        &mut self,
        id: Uuid,
        name: String,
        spec: ProgramSpec,
    ) -> Task<Message> {
        self.change_collection(move |collection| {
            collection.programs.insert(id, ProgramRecord { name, spec });
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

/// The path Wine reaches host path `path` by, through the `Z:` drive that
/// maps the host root.
fn wine_path(path: &Path) -> String {
    format!("Z:{}", path.display()).replace('/', "\\")
}

/// How Wine launches host file `executable`: from its own folder, with the
/// Launch Options' words as arguments. `%command%` stands for the program,
/// as in Steam's launch options, and isn't passed on.
fn program_spec(executable: &Path, options: &str) -> ProgramSpec {
    let spec = ProgramSpec::new(wine_path(executable)).with_args(
        options
            .split_whitespace()
            .filter(|word| *word != "%command%")
            .map(str::to_owned),
    );
    match executable.parent() {
        Some(folder) => spec.with_working_directory(wine_path(folder)),
        None => spec,
    }
}
