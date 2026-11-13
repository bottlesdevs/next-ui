//! Operations in flight, each keyed by what it acts on.

use std::{collections::HashMap, future::Future, pin::pin};

use bottles_core::{Operation, Progress, error};
use iced::{
    Task,
    futures::{Stream, StreamExt as _, future, stream},
    task::sipper,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;

use super::Action;

/// What an operation acts on. A subject has at most one operation at a time.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Subject {
    Catalog,
    Addon(Uuid),
    Prefix(Uuid),
    Program {
        prefix: Uuid,
        group: Uuid,
    },
    Entry {
        plugin: String,
        id: String,
    },
    /// A bottle being created, under an identifier made up until it exists.
    Creation(Uuid),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verb {
    Refresh,
    /// Downloads a release.
    Fetch,
    /// Creates a prefix.
    Create,
    Edit,
    Launch,
    Stop,
    Delete,
}

pub(crate) struct Pending {
    pub(crate) verb: Verb,
    pub(crate) progress: Option<Progress>,
    /// What to do once the operation succeeds, e.g. select the release it
    /// downloaded. Dropped when it fails or is cancelled.
    pub(crate) then: Option<Action>,
    cancel: CancellationToken,
}

/// What a finished operation leaves for the model to act on.
#[derive(Clone, Debug)]
pub(crate) enum Outcome {
    Done,
    /// Stopped before it finished, e.g. a download the user cancelled.
    Cancelled,
    /// The prefix of a created bottle.
    Created(Uuid),
}

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Progress(Progress),
    Finished(Result<Outcome, String>),
}

/// An operation that finished, with the continuation it carried.
pub(crate) struct Finished {
    pub(crate) verb: Verb,
    pub(crate) then: Option<Action>,
    pub(crate) result: Result<Outcome, String>,
}

pub(crate) struct Operations {
    pending: HashMap<Subject, Pending>,
    tracker: TaskTracker,
    shutdown: CancellationToken,
}

impl Operations {
    /// Creates a registry whose operations `tracker` waits for at shutdown.
    pub(crate) fn new(tracker: TaskTracker) -> Self {
        Self {
            pending: HashMap::new(),
            tracker,
            shutdown: CancellationToken::new(),
        }
    }

    pub(crate) fn get(&self, subject: &Subject) -> Option<&Pending> {
        self.pending.get(subject)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Subject, &Pending)> {
        self.pending.iter()
    }

    /// Starts `operation` on `subject`, unless the subject already has one,
    /// to be followed by `then` once it succeeds.
    pub(crate) fn start<T: Send + 'static>(
        &mut self,
        subject: Subject,
        verb: Verb,
        then: Option<Action>,
        operation: Operation<T>,
        outcome: impl FnOnce(T) -> Outcome + Send + 'static,
    ) -> Task<(Subject, Event)> {
        let cancel = operation.cancellation_token();
        let progress = operation.progress();
        let shutdown = self.shutdown.clone();
        let operation_cancel = cancel.clone();
        let drive = async move {
            let result =
                match future::select(Box::pin(shutdown.cancelled_owned()), Box::pin(operation))
                    .await
                {
                    future::Either::Left(((), operation)) => {
                        operation_cancel.cancel();
                        operation.await
                    }
                    future::Either::Right((result, _)) => result,
                };
            match result {
                Ok(value) => Ok(outcome(value)),
                Err(error::Error::Cancelled) => Ok(Outcome::Cancelled),
                Err(error) => Err(error.to_string()),
            }
        };
        self.run(subject, verb, then, cancel, progress, drive)
    }

    /// Runs `future` on `subject`, unless the subject already has an operation.
    pub(crate) fn perform(
        &mut self,
        subject: Subject,
        verb: Verb,
        future: impl Future<Output = Result<(), String>> + Send + 'static,
    ) -> Task<(Subject, Event)> {
        let cancel = self.shutdown.child_token();
        let drive = {
            let cancel = cancel.clone();
            async move {
                let result = cancel.run_until_cancelled(future).await.unwrap_or(Ok(()));
                result.map(|()| Outcome::Done)
            }
        };
        self.run(subject, verb, None, cancel, stream::empty(), drive)
    }

    fn run(
        &mut self,
        subject: Subject,
        verb: Verb,
        then: Option<Action>,
        cancel: CancellationToken,
        progress: impl Stream<Item = Progress> + Send + 'static,
        drive: impl Future<Output = Result<Outcome, String>> + Send + 'static,
    ) -> Task<(Subject, Event)> {
        if self.pending.contains_key(&subject) {
            return Task::none();
        }
        self.pending.insert(
            subject.clone(),
            Pending {
                verb,
                progress: None,
                then,
                cancel,
            },
        );
        // Register now: shutdown must also wait for tasks not yet polled by Iced.
        let drive = self.tracker.track_future(drive);
        let finished = subject.clone();
        Task::sip(
            sipper(move |mut sender| async move {
                let report = async move {
                    let mut progress = pin!(progress);
                    while let Some(update) = progress.next().await {
                        sender.send(update).await;
                    }
                };
                future::join(drive, report).await.0
            }),
            move |progress| (subject.clone(), Event::Progress(progress)),
            move |result| (finished, Event::Finished(result)),
        )
    }

    /// Records `event`, returning the operation it finished, if any.
    pub(crate) fn update(&mut self, subject: &Subject, event: Event) -> Option<Finished> {
        match event {
            Event::Progress(progress) => {
                if let Some(pending) = self.pending.get_mut(subject) {
                    pending.progress = Some(progress);
                }
                None
            }
            Event::Finished(result) => {
                let pending = self.pending.remove(subject)?;
                Some(Finished {
                    verb: pending.verb,
                    then: pending.then,
                    result,
                })
            }
        }
    }

    pub(crate) fn cancel(&self, subject: &Subject) {
        if let Some(pending) = self.pending.get(subject) {
            pending.cancel.cancel();
        }
    }

    /// Cancels every operation, so that the tracker can be waited for.
    pub(crate) fn shutdown(&self) {
        self.shutdown.cancel();
        for pending in self.pending.values() {
            pending.cancel.cancel();
        }
    }
}
