//! Bottle "Snapshots" detail tab. The Classic workspace loads it whenever a
//! bottle is selected and maps the task back into its own message enum.

use std::{collections::HashMap, sync::Arc};

use bottles_core::{Bottle, SnapshotSummary, error::Error as CoreError};
use iced::{
    Element, Fill, Length, Task,
    widget::{column, responsive},
};
use tokio_util::sync::CancellationToken;

use crate::{
    icons::Icon,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        info_card::{InfoCard, Kind},
        row_group::RowGroup,
    },
};

use super::CONTENT_GRID_BREAKPOINT;

#[derive(Clone)]
pub enum Message {
    Loaded {
        generation: u64,
        result: Option<Result<Vec<SnapshotSummary>, Arc<CoreError>>>,
    },
    Noop,
}

#[derive(Default)]
pub struct State {
    snapshots: Vec<SnapshotSummary>,
    snapshot_rows: Vec<(String, String)>,
    generation: u64,
    loads: HashMap<u64, CancellationToken>,
    last_error: Option<Arc<CoreError>>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.cancel_active_operations();
        self.snapshots.clear();
        self.snapshot_rows.clear();
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
        let Message::Loaded { generation, result } = message else {
            return Task::none();
        };
        self.loads.remove(&generation);
        if generation == self.generation {
            match result {
                Some(Ok(snapshots)) => {
                    self.snapshot_rows = snapshots
                        .iter()
                        .map(|snapshot| {
                            let title = if snapshot.message.is_empty() {
                                snapshot.state_id.chars().take(12).collect()
                            } else {
                                snapshot.message.clone()
                            };
                            let description = snapshot
                                .created_at
                                .as_ref()
                                .map(|timestamp| relative_time(timestamp.seconds))
                                .unwrap_or_default();

                            (title, description)
                        })
                        .collect();
                    self.snapshots = snapshots;
                    self.last_error = None;
                }
                Some(Err(error)) => self.last_error = Some(error),
                None => {}
            }
        }

        Task::none()
    }

    pub fn has_active_operation(&self) -> bool {
        !self.loads.is_empty()
    }

    pub fn cancel_active_operations(&self) {
        for cancellation in self.loads.values() {
            cancellation.cancel();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rows = responsive(move |size| {
            let columns = usize::from(size.width >= CONTENT_GRID_BREAKPOINT) + 1;

            self.snapshot_rows
                .iter()
                .fold(
                    RowGroup::new().columns(columns),
                    |rows, (title, description)| {
                        rows.row(
                            ActionRow::new(title, ActionRowState::Ready(Message::Noop))
                                .description(description)
                                .icon(Icon::Timer),
                        )
                    },
                )
                .into()
        })
        .height(Length::Shrink);

        let mut content = column![rows].spacing(12);
        if let Some(error) = &self.last_error {
            content = content.push(
                InfoCard::new(Kind::Error, "Could not load snapshots", error.to_string())
                    .width(Fill),
            );
        }

        content.into()
    }
}

fn relative_time(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(seconds, |duration| duration.as_secs() as i64);
    let diff = (now - seconds).max(0);

    match diff {
        0..=59 => "Just now".to_string(),
        60..=3599 => format!("{} minutes ago", diff / 60),
        3600..=86399 => format!("{} hours ago", diff / 3600),
        _ => format!("{} days ago", diff / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_stays_active_until_the_terminal_message() {
        let mut state = State::new();
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
