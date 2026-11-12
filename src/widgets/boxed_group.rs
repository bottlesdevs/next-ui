//! A list of rows joined into one block, like libadwaita's boxed list.

use iced::{
    Alignment, Border, Element, Fill,
    border::Radius,
    widget::{Column, container, row, text},
};

use crate::icons::Icon;

use super::{action_row::ActionRow, list_row::ListRow, spacing, text::TextExt as _};

/// The block's outer corners.
const RADIUS: f32 = 8.0;
/// The hairline between joined rows.
const GAP: f32 = 2.0;

/// Rows joined into one block under an optional heading row: only the
/// block's outer corners are rounded, and the rows sit a hairline apart.
pub struct BoxedGroup<'a, Message> {
    title: Option<&'a str>,
    icon: Option<Icon>,
    rows: Vec<Box<dyn FnOnce(Radius) -> Element<'a, Message> + 'a>>,
}

/// A row a [`BoxedGroup`] can join, taking the corners its place gives it.
pub trait GroupRow<'a, Message> {
    fn into_group_row(self, corners: Radius) -> Element<'a, Message>;
}

impl<'a, Message: 'a> GroupRow<'a, Message> for ListRow<'a, Message> {
    fn into_group_row(self, corners: Radius) -> Element<'a, Message> {
        self.corners(corners).into()
    }
}

impl<'a, Message: Clone + 'a> GroupRow<'a, Message> for ActionRow<'a, Message> {
    fn into_group_row(self, corners: Radius) -> Element<'a, Message> {
        self.corners(corners).into()
    }
}

impl<'a, Message: 'a> BoxedGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            icon: None,
            rows: Vec::new(),
        }
    }

    /// Heads the block with a row naming it.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Shows `icon` before the title.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn row(mut self, row: impl GroupRow<'a, Message> + 'a) -> Self {
        self.rows
            .push(Box::new(move |corners| row.into_group_row(corners)));
        self
    }
}

impl<'a, Message: 'a> Default for BoxedGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<BoxedGroup<'a, Message>> for Element<'a, Message> {
    fn from(group: BoxedGroup<'a, Message>) -> Self {
        let count = group.rows.len() + usize::from(group.title.is_some());
        let corners = |index: usize| Radius {
            top_left: if index == 0 { RADIUS } else { 0.0 },
            top_right: if index == 0 { RADIUS } else { 0.0 },
            bottom_left: if index + 1 == count { RADIUS } else { 0.0 },
            bottom_right: if index + 1 == count { RADIUS } else { 0.0 },
        };
        let mut block = Column::new().spacing(GAP).width(Fill);
        let mut index = 0;
        if let Some(title) = group.title {
            let mut heading = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if let Some(icon) = group.icon {
                heading = heading.push(icon.view().width(18).height(18));
            }
            heading = heading.push(text(title).size(15).medium());
            let radius = corners(index);
            block = block.push(
                container(heading)
                    .width(Fill)
                    .padding([spacing::SM, spacing::LG])
                    .style(move |theme| {
                        let colors = crate::theme::colors(theme);
                        container::Style::default()
                            .background(colors.surface)
                            .color(colors.accent_muted)
                            .border(Border::default().rounded(radius))
                    }),
            );
            index += 1;
        }
        for row in group.rows {
            block = block.push(row(corners(index)));
            index += 1;
        }
        block.into()
    }
}
