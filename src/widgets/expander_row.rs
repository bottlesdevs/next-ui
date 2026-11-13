use iced::{Element, widget::text::IntoFragment};

use super::{list_row::ListRow, row_group::standalone_expander};

/// A row that discloses more rows beneath it, like `AdwExpanderRow`.
pub struct ExpanderRow<'a, Message> {
    pub(crate) header: ListRow<'a, Message>,
    pub(crate) columns: usize,
    pub(crate) content: Vec<Element<'a, Message>>,
}

impl<'a, Message: 'a> ExpanderRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self::with_header(ListRow::labeled(title).subtitle(""))
    }

    pub fn with_header(header: impl Into<ListRow<'a, Message>>) -> Self {
        Self {
            header: header.into(),
            columns: 1,
            content: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.header = self.header.subtitle(description);
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn row(mut self, row: impl Into<Element<'a, Message>>) -> Self {
        self.content.push(row.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        standalone_expander(expander)
    }
}
