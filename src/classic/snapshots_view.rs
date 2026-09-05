use iced::{
    Element, Fill, Length,
    widget::{column, responsive},
};

use crate::{
    Icon,
    features::snapshots::State,
    widget::{ActionRow, InfoCard, InfoCardKind, RowGroup},
};

const GRID_BREAKPOINT: f32 = 720.0;

#[derive(Clone, Copy)]
pub enum Message {
    Selected,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let rows = responsive(move |size| {
        let columns = usize::from(size.width >= GRID_BREAKPOINT) + 1;
        state
            .snapshots()
            .iter()
            .fold(RowGroup::new().columns(columns), |rows, snapshot| {
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

                rows.row(
                    ActionRow::new(title, Message::Selected)
                        .description(description)
                        .icon(Icon::Timer),
                )
            })
            .into()
    })
    .height(Length::Shrink);

    let mut content = column![rows].spacing(12);
    if let Some(error) = state.last_error() {
        content = content.push(
            InfoCard::new(
                InfoCardKind::Error,
                "Could not load snapshots",
                error.to_string(),
            )
            .width(Fill),
        );
    }

    content.into()
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
