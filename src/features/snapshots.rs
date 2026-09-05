use std::{collections::HashMap, sync::Arc};

use bottles_core::{Bottle, SnapshotSummary, error::Error as CoreError};
use iced::Task;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub enum Message {
    Loaded {
        generation: u64,
        result: Option<Result<Vec<SnapshotSummary>, Arc<CoreError>>>,
    },
}

#[derive(Default)]
pub struct State {
    snapshots: Vec<SnapshotSummary>,
    generation: u64,
    loads: HashMap<u64, CancellationToken>,
    last_error: Option<Arc<CoreError>>,
}

impl State {
    pub fn clear(&mut self) {
        self.cancel_active_operations();
        self.snapshots.clear();
        self.last_error = None;
    }

    pub fn load(&mut self, bottle: Bottle) -> Task<Message> {
        self.clear();
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        self.loads.insert(generation, cancellation);

        Task::perform(
            async move {
                task_cancellation
                    .run_until_cancelled(bottle.snapshots())
                    .await
                    .map(|result| result.map_err(Arc::new))
            },
            move |result| Message::Loaded { generation, result },
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Loaded { generation, result } = message;
        self.loads.remove(&generation);
        if generation == self.generation {
            match result {
                Some(Ok(snapshots)) => {
                    self.snapshots = snapshots;
                    self.last_error = None;
                }
                Some(Err(error)) => self.last_error = Some(error),
                None => {}
            }
        }

        Task::none()
    }

    pub fn snapshots(&self) -> &[SnapshotSummary] {
        &self.snapshots
    }

    pub fn last_error(&self) -> Option<&CoreError> {
        self.last_error.as_deref()
    }

    pub fn has_active_operation(&self) -> bool {
        !self.loads.is_empty()
    }

    pub fn cancel_active_operations(&self) {
        for cancellation in self.loads.values() {
            cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_stays_active_until_the_terminal_message() {
        let mut state = State::default();
        let cancellation = CancellationToken::new();
        state.loads.insert(1, cancellation.clone());

        state.cancel_active_operations();

        assert!(cancellation.is_cancelled());
        assert!(state.has_active_operation());

        let _ = state.update(Message::Loaded {
            generation: 1,
            result: None,
        });
        assert!(!state.has_active_operation());
    }
}
