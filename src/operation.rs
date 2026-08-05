use bottles_core::{Operation, error};
use iced::{
    Task,
    futures::{FutureExt as _, SinkExt as _, StreamExt as _, pin_mut, select},
};

#[derive(Debug, Clone)]
pub enum Event<K, T, P> {
    Progress { key: K, progress: P },
    Finished { key: K, outcome: Outcome<T> },
}

#[derive(Debug, Clone)]
pub enum Outcome<T> {
    Succeeded(T),
    Cancelled,
    Failed(String),
}

pub trait OperationExt: Sized {
    type Output: Send + 'static;
    type Progress: Clone + Send + Sync + 'static;

    /// Runs the operation and emits messages tagged with this specific run's key.
    /// Include a generation in the key when the same business entity can be run again.
    fn run<K>(self, key: K) -> Task<Event<K, Self::Output, Self::Progress>>
    where
        K: Clone + Send + 'static;
}

impl<T, P> OperationExt for Operation<T, P>
where
    T: Send + 'static,
    P: Clone + Send + Sync + 'static,
{
    type Output = T;
    type Progress = P;

    fn run<K>(self, key: K) -> Task<Event<K, T, P>>
    where
        K: Clone + Send + 'static,
    {
        let progress = self.progress();
        let progress_key = key.clone();
        let events = iced::stream::channel(1, async move |mut output| {
            let progress = progress.fuse();
            let terminal = self.map(outcome).fuse();
            pin_mut!(progress, terminal);

            loop {
                select! {
                    outcome = terminal => {
                        let _ = output.send(Event::Finished { key, outcome }).await;
                        break;
                    }
                    update = progress.next() => match update {
                        Some(progress) => {
                            if output.send(Event::Progress {
                                key: progress_key.clone(),
                                progress,
                            }).await.is_err() {
                                break;
                            }
                        }
                        None => {
                            let outcome = terminal.await;
                            let _ = output.send(Event::Finished { key, outcome }).await;
                            break;
                        }
                    }
                }
            }
        });

        Task::run(events, std::convert::identity)
    }
}

fn outcome<T>(result: error::Result<T>) -> Outcome<T> {
    match result {
        Ok(value) => Outcome::Succeeded(value),
        Err(error::Error::Cancelled) => Outcome::Cancelled,
        Err(error) => Outcome::Failed(error.to_string()),
    }
}
