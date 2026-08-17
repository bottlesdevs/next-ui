//! Bottle "Snapshots" detail tab. Loaded whenever the shell selects a
//! bottle — the shell drives [`State::load`] from `BottleSelected` and maps
//! the resulting task into `shell::Message::Snapshots`.

use bottles_core::{Bottle, SnapshotSummary};
use iced::{
    Element, Task,
    widget::container,
};

use crate::{
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        row_group::RowGroup,
    },
    icons::Icon,
};

const CONTENT_GRID_BREAKPOINT: f32 = 720.0;

#[derive(Clone)]
pub enum Message {
    Loaded(Result<Vec<SnapshotSummary>, String>),
    Noop,
}

#[derive(Default)]
pub struct State {
    snapshots: Vec<SnapshotSummary>,
    snapshot_rows: Vec<(String, String)>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.snapshot_rows.clear();
    }

    pub fn load(&mut self, bottle: Bottle) -> Task<Message> {
        self.clear();

        Task::perform(
            async move { bottle.snapshots().await.map_err(|err| err.to_string()) },
            Message::Loaded,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(snapshots)) => {
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
            }
            Message::Loaded(Err(err)) => eprintln!("failed to load snapshots: {err}"),
            Message::Noop => {}
        }

        Task::none()
    }

    pub fn view(&self, width: f32) -> Element<'_, Message> {
        let columns = usize::from(width >= CONTENT_GRID_BREAKPOINT) + 1;
        let rows = self.snapshot_rows.iter().fold(
            RowGroup::new().columns(columns),
            |rows, (title, description)| {
                rows.add(
                    ActionRow::new(title, ActionRowState::Ready(Message::Noop))
                        .description(description)
                        .icon(Icon::Timer),
                )
            },
        );

        container(rows).max_width(1150).into()
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
