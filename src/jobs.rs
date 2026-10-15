use std::{collections::HashMap, future::Future, path::Path, pin::pin};

use bottles_core::{Operation, Progress, error};
use iced::{
    Task,
    futures::{Stream, StreamExt as _, channel::oneshot, future, stream},
    task::sipper,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;

use crate::{app::Message, settings::Settings};

#[derive(Default)]
pub(crate) struct Jobs {
    pub(crate) map: HashMap<Uuid, Job>,
    tracker: TaskTracker,
    shutdown: CancellationToken,
    save_tail: Option<oneshot::Receiver<()>>,
}

pub(crate) struct Job {
    pub(crate) kind: Kind,
    pub(crate) progress: Option<Progress>,
    pub(crate) cancel: CancellationToken,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    RefreshCatalog,
    Fetch(Uuid),
    CreateBottle(String),
    Edit(Uuid),
    Launch(String, String),
    Stop(Uuid),
    Delete(Uuid),
    SaveSettings,
}

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Progress(Progress),
    Finished(Result<(), String>),
}

impl Jobs {
    pub(crate) fn running(&self, kind: &Kind) -> bool {
        self.find(kind).is_some()
    }

    pub(crate) fn find(&self, kind: &Kind) -> Option<&Job> {
        self.map.values().find(|job| &job.kind == kind)
    }

    pub(crate) fn start<T: Send + 'static>(
        &mut self,
        kind: Kind,
        operation: Operation<T>,
    ) -> Task<Message> {
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
                Ok(_) | Err(error::Error::Cancelled) => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        };
        self.run(kind, cancel, progress, drive)
    }

    pub(crate) fn save_settings(&mut self, settings: Settings, config_dir: &Path) -> Task<Message> {
        let save = settings.save(config_dir);
        let (completion, next) = oneshot::channel();
        let previous = self.save_tail.replace(next);
        // Settings writes finish in invocation order, including during shutdown.
        let drive = async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let result = save.await;
            drop(completion);
            result
        };
        self.run(
            Kind::SaveSettings,
            CancellationToken::new(),
            stream::empty(),
            drive,
        )
    }

    fn run(
        &mut self,
        kind: Kind,
        cancel: CancellationToken,
        progress: impl Stream<Item = Progress> + Send + 'static,
        drive: impl Future<Output = Result<(), String>> + Send + 'static,
    ) -> Task<Message> {
        let id = Uuid::new_v4();
        self.map.insert(
            id,
            Job {
                kind,
                progress: None,
                cancel,
            },
        );
        // Register now: shutdown must also wait for tasks not yet polled by Iced.
        let drive = self.tracker.track_future(drive);
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
            move |progress| Message::Job(id, Event::Progress(progress)),
            move |result| Message::Job(id, Event::Finished(result)),
        )
    }

    pub(crate) fn update(&mut self, id: Uuid, event: Event) -> Option<String> {
        match event {
            Event::Progress(progress) => {
                if let Some(job) = self.map.get_mut(&id) {
                    job.progress = Some(progress);
                }
                None
            }
            Event::Finished(result) => {
                self.map.remove(&id);
                result.err()
            }
        }
    }

    pub(crate) fn shutdown(&self) -> impl Future<Output = ()> + Send + 'static {
        self.shutdown.cancel();
        for job in self.map.values() {
            job.cancel.cancel();
        }
        let tracker = self.tracker.clone();
        tracker.close();
        async move { tracker.wait().await }
    }
}
