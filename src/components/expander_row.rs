use iced::{
    Border, ContentFit, Element, Theme,
    widget::{button, svg},
};

use super::{
    list_row::{HoverTone, ListRow, labels},
    pressable::{Pressable, Status},
    row_group::{RowGroup, standalone_expander},
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
    enabled: bool,
}

pub(crate) struct ExpanderParts<'a, Message> {
    pub header: ListRow<'a, Message>,
    pub expanded: bool,
    pub content: Option<Element<'a, Message>>,
}

impl<'a, Message: 'a> ExpanderRow<'a, Message> {
    pub fn new(title: &'a str, expanded: bool, on_toggle: Message) -> Self {
        Self {
            title,
            description: "",
            header: None,
            expanded,
            on_toggle,
            columns: 1,
            content: Vec::new(),
            content_enabled: true,
            enabled: true,
        }
    }

    pub fn with_header(
        header: impl Into<ListRow<'a, Message>>,
        expanded: bool,
        on_toggle: Message,
    ) -> Self {
        Self {
            header: Some(header.into()),
            ..Self::new("", expanded, on_toggle)
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
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

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a, Message: Clone + 'a> ExpanderRow<'a, Message> {
    pub(crate) fn into_parts(self) -> ExpanderParts<'a, Message> {
        let content = if self.content.is_empty() {
            None
        } else {
            Some(
                self.content
                    .into_iter()
                    .fold(
                        RowGroup::new()
                            .columns(self.columns)
                            .enabled(self.content_enabled),
                        RowGroup::add,
                    )
                    .into(),
            )
        };
        let expanded = self.expanded && content.is_some();
        let caret = || {
            svg(crate::icons::Icon::DownCaret.handle())
                .width(20)
                .height(20)
                .content_fit(ContentFit::Contain)
                .rotation(if expanded { std::f32::consts::PI } else { 0.0 })
        };
        let header = match (self.header, content.is_some()) {
            (Some(header), true) => header.prepend_trailing(
                Pressable::new(caret())
                    .padding(6)
                    .on_press(self.on_toggle)
                    .style(caret_style),
            ),
            (Some(header), false) => header,
            (None, true) => ListRow::new(labels(self.title, self.description))
                .trailing(caret())
                .on_press(self.on_toggle),
            (None, false) => ListRow::new(labels(self.title, self.description)),
        }
        .raised(expanded)
        .enabled(self.enabled);

        ExpanderParts {
            header,
            expanded,
            content,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        standalone_expander(expander.into_parts())
    }
}

fn caret_style(theme: &Theme, _status: Status) -> button::Style {
    button::Style {
        text_color: theme.palette().text,
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        action_row::{ActionRow, ActionRowState},
        switcher_row::SwitcherRow,
    };

    use super::ExpanderRow;

    #[test]
    fn accepts_an_interactive_row_as_its_header() {
        let expander =
            ExpanderRow::with_header(SwitcherRow::new("Switch", false, |_| ()), false, ())
                .columns(2)
                .add(ActionRow::new("child", ActionRowState::Ready(())));

        assert!(expander.header.is_some());
        assert_eq!(expander.columns, 2);
        assert_eq!(expander.content.len(), 1);
    }

    #[test]
    fn empty_expanders_do_not_expand() {
        assert!(!ExpanderRow::new("Empty", true, ()).into_parts().expanded);
    }
}
