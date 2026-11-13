//! What happened in the model that the user may need to hear about, as data
//! for the shell to word.

use super::{
    Action,
    operations::{Outcome, Subject, Verb},
};

#[derive(Debug)]
pub(crate) enum Report {
    /// Installed plugin `id` failed to load.
    PluginFailed { id: String, error: String },
    /// The operation on `subject` finished. `then` is its continuation, which
    /// was performed only if it succeeded.
    Finished {
        subject: Subject,
        verb: Verb,
        then: Option<Action>,
        result: Result<Outcome, String>,
    },
    /// An action wasn't started.
    Refused(Refusal),
    /// Core, a plugin, or a file the application owns failed.
    Error(String),
}

/// Why an action wasn't started.
#[derive(Debug)]
pub(crate) enum Refusal {
    /// The release to select is no longer downloaded.
    ReleaseGone,
    /// A Proton runner was chosen but UMU isn't downloaded.
    NoUmu,
    /// A suggestion can't be installed without bottle recipes.
    NoRecipes(&'static str),
    /// No runner is downloaded to give a standalone program.
    NoRunner,
    /// Creating a prefix needs a runner and WineBridge.
    NoRunnerOrWineBridge,
    /// The program to launch no longer exists.
    ProgramGone,
    /// The plugin of the entry to launch is no longer loaded.
    PluginGone,
}
