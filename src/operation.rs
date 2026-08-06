use bottles_core::{Operation, Progress, error};
use iced::Task;

#[derive(Debug, Clone)]
pub enum Event<K, T> {
    Progress { key: K, progress: Progress },
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
    /// Runs the operation and emits messages tagged with this specific run's key.
    /// Include a generation in the key when the same business entity can be run again.
    fn run<K>(self, key: K) -> Task<Event<K, Self::Output>>
    where
        K: Clone + Send + 'static;
}

impl<T> OperationExt for Operation<T>
where
    T: Send + 'static,
{
    type Output = T;

    fn run<K>(self, key: K) -> Task<Event<K, T>>
    where
        K: Clone + Send + 'static,
    {
        let progress_key = key.clone();
        let progress = Task::run(self.progress(), move |progress| Event::Progress {
            key: progress_key.clone(),
            progress,
        });
        let finished = Task::perform(self, move |result| Event::Finished {
            key,
            outcome: outcome(result),
        });

        Task::batch([progress, finished])
    }
}

fn outcome<T>(result: error::Result<T>) -> Outcome<T> {
    match result {
        Ok(value) => Outcome::Succeeded(value),
        Err(error::Error::Cancelled) => Outcome::Cancelled,
        Err(error) => Outcome::Failed(error.to_string()),
    }
}
