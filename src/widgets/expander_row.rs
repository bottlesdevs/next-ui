use iced::Element;

use super::{list_row::ListRow, row_group::standalone_expander};

pub struct ExpanderRow<'a, Message> {
    pub(crate) header: Header<'a, Message>,
    pub(crate) columns: usize,
    pub(crate) content: Vec<ListRow<'a, Message>>,
    pub(crate) content_enabled: bool,
    pub(crate) enabled: bool,
}

pub(crate) enum Header<'a, Message> {
    Labels {
        title: &'a str,
        description: &'a str,
    },
    Custom(ListRow<'a, Message>),
}

impl<'a, Message> ExpanderRow<'a, Message> {
    pub fn new(title: &'a str) -> Self {
        Self {
            header: Header::Labels {
                title,
                description: "",
            },
            columns: 1,
            content: Vec::new(),
            content_enabled: true,
            enabled: true,
        }
    }

    pub fn with_header(header: impl Into<ListRow<'a, Message>>) -> Self {
        Self {
            header: Header::Custom(header.into()),
            ..Self::new("")
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        if let Header::Labels {
            description: current,
            ..
        } = &mut self.header
        {
            *current = description;
        }

        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn add(mut self, row: impl Into<ListRow<'a, Message>>) -> Self {
        self.content.push(row.into());
        self
    }

    pub fn content_enabled(mut self, enabled: bool) -> Self {
        self.content_enabled = enabled;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        standalone_expander(expander)
    }
}

pub(crate) fn header_row<'a, Message: 'a>(
    header: Header<'a, Message>,
    enabled: bool,
) -> ListRow<'a, Message> {
    match header {
        Header::Labels { title, description } => {
            ListRow::new(super::list_row::labels(title, description))
        }
        Header::Custom(header) => header,
    }
    .enabled(enabled)
}
