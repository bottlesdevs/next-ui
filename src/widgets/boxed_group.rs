//! A list of rows joined into one block, like libadwaita's boxed list.

use iced::{Element, widget::text::IntoFragment};

use crate::icons::Icon;

use super::{list_row::ListRow, row_group::RowGroup};

/// Rows joined into one block under an optional heading row: the boxed
/// style of [`RowGroup`].
pub struct BoxedGroup<'a, Message>(RowGroup<'a, Message>);

impl<'a, Message: 'a> BoxedGroup<'a, Message> {
    pub fn new() -> Self {
        Self(RowGroup::boxed())
    }

    /// Heads the block with a row naming it.
    pub fn title(self, title: impl IntoFragment<'a>) -> Self {
        Self(self.0.title(title))
    }

    /// Shows `icon` before the title.
    pub fn icon(self, icon: Icon) -> Self {
        Self(self.0.icon(icon))
    }

    pub fn row(self, row: impl Into<ListRow<'a, Message>>) -> Self {
        Self(self.0.list_row(row))
    }
}

impl<'a, Message: 'a> Default for BoxedGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<BoxedGroup<'a, Message>> for Element<'a, Message> {
    fn from(group: BoxedGroup<'a, Message>) -> Self {
        group.0.into()
    }
}
