use iced::{
    ContentFit, Element, Fill,
    widget::{button, container, svg},
};

use super::{
    list_row::{ListRow, labels},
    row_group::RowGroup,
    style,
};

pub struct ExpanderRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    header: Option<ListRow<'a, Message>>,
    expanded: bool,
    on_toggle: Message,
    content: Option<RowGroup<'a, Message>>,
    content_enabled: bool,
}

pub(crate) struct ExpanderParts<'a, Message> {
    pub header: ListRow<'a, Message>,
    pub expanded: bool,
    pub content: Option<Element<'a, Message>>,
}

impl<'a, Message> ExpanderRow<'a, Message> {
    pub fn new(on_toggle: Message) -> Self {
        Self {
            title: "",
            description: "",
            header: None,
            expanded: false,
            on_toggle,
            content: None,
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

    pub fn content(mut self, content: impl Into<RowGroup<'a, Message>>) -> Self {
        self.content = Some(content.into());
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
            svg(crate::icons::get("down_caret"))
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

        ExpanderParts {
            header,
            expanded,
            content: self
                .content
                .map(|content| content.enabled(self.content_enabled).into()),
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
            .header(SwitcherRow::new(false, |_| ()))
            .content(ActionRow::new().title("child"));

        assert!(expander.header.is_some());
        assert!(expander.content.is_some());
    }
}
