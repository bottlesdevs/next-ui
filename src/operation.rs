use std::sync::Arc;

use bottles_core::{Operation, Progress, error};
use iced::{
    Task,
    futures::{SinkExt as _, StreamExt as _, future},
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum Event<K, T> {
    Progress { key: K, progress: Progress },
    Finished { key: K, outcome: Outcome<T> },
}

#[derive(Debug, Clone)]
pub enum Outcome<T> {
    Succeeded(T),
    Cancelled,
    Failed(Arc<error::Error>),
}

/// Drives an operation to completion while reporting progress for one keyed run.
///
/// Cancelling the returned token only requests cancellation. Keep the task alive
/// until it emits [`Event::Finished`] so the operation can perform its cleanup.
pub fn run<K, T>(operation: Operation<T>, key: K) -> (CancellationToken, Task<Event<K, T>>)
where
    T: Send + 'static,
    K: Clone + Send + 'static,
{
    let cancellation = operation.cancellation_token();
    let events = iced::stream::channel(
        16,
        move |mut output: iced::futures::channel::mpsc::Sender<Event<K, T>>| async move {
            let mut progress = operation.progress();
            let progress_key = key.clone();
            let mut progress_output = output.clone();
            let report_progress = async move {
                while let Some(progress) = progress.next().await {
                    if progress_output
                        .send(Event::Progress {
                            key: progress_key.clone(),
                            progress,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            };

            let (result, ()) = future::join(operation, report_progress).await;
            let _ = output
                .send(Event::Finished {
                    key,
                    outcome: outcome(result),
                })
                .await;
        },
    );

    (cancellation, Task::run(events, std::convert::identity))
}

fn outcome<T>(result: error::Result<T>) -> Outcome<T> {
    match result {
        Ok(value) => Outcome::Succeeded(value),
        Err(error::Error::Cancelled) => Outcome::Cancelled,
        Err(error) => Outcome::Failed(Arc::new(error)),
    }
}
