use iced::{
    ContentFit, Element, Fill,
    widget::{button, container, svg},
};

use super::{
    list_row::{HoverTone, ListRow, labels},
    row_group::RowGroup,
    style,
};

pub struct ExpanderRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    header: Option<ListRow<'a, Message>>,
    expanded: bool,
    on_toggle: Message,
    columns: usize,
    content: Vec<ListRow<'a, Message>>,
    content_enabled: bool,
}

pub(crate) struct ExpanderParts<'a, Message> {
    pub header: ListRow<'a, Message>,
    pub expanded: bool,
    pub content: Option<Element<'a, Message>>,
}

impl<'a, Message: 'a> ExpanderRow<'a, Message> {
    pub fn new(on_toggle: Message) -> Self {
        Self {
            title: "",
            description: "",
            header: None,
            expanded: false,
            on_toggle,
            columns: 1,
            content: Vec::new(),
            content_enabled: true,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn header(mut self, header: impl Into<ListRow<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn add(mut self, row: impl Into<ListRow<'a, Message>>) -> Self {
        let mut row = row.into();
        row.set_hover_tone(HoverTone::Strong);
        self.content.push(row);
        self
    }

    pub fn content_enabled(mut self, enabled: bool) -> Self {
        self.content_enabled = enabled;
        self
    }
}

impl<'a, Message: Clone + 'a> ExpanderRow<'a, Message> {
    pub(crate) fn into_parts(self) -> ExpanderParts<'a, Message> {
        let expanded = self.expanded;
        let caret = || {
            svg(crate::icons::Icon::DownCaret.handle())
                .width(20)
                .height(20)
                .content_fit(ContentFit::Contain)
                .rotation(if expanded { std::f32::consts::PI } else { 0.0 })
        };

        let header = match self.header {
            Some(header) => header.prepend_trailing(
                button(caret())
                    .padding(6)
                    .on_press(self.on_toggle)
                    .style(style::tab),
            ),
            None => ListRow::new(labels(self.title, self.description))
                .trailing(caret())
                .on_press(self.on_toggle),
        }
        .raised(expanded);

        let content = if self.content.is_empty() {
            None
        } else {
            let content = self.content.into_iter().fold(
                RowGroup::new()
                    .columns(self.columns)
                    .enabled(self.content_enabled),
                RowGroup::add,
            );

            Some(content.into())
        };

        ExpanderParts {
            header,
            expanded,
            content,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        let parts = expander.into_parts();
        let mut row = parts.header;

        if parts.expanded
            && let Some(content) = parts.content
        {
            row = row.content(container(content).width(Fill).padding(18));
        }

        row.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{action_row::ActionRow, switcher_row::SwitcherRow};

    use super::ExpanderRow;

    #[test]
    fn accepts_an_interactive_row_as_its_header() {
        let expander = ExpanderRow::new(())
            .header(SwitcherRow::new("Switch", false, |_| ()))
            .columns(2)
            .add(ActionRow::new().title("child"));

        assert!(expander.header.is_some());
        assert_eq!(expander.columns, 2);
        assert_eq!(expander.content.len(), 1);
    }
}
