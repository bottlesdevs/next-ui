use iced::{
    Alignment, Background, Border, ContentFit, Element, Fill, Theme,
    widget::{button, column, container, row, svg, text},
};

use super::{row_surface::RowSurface, style};

pub struct ExpanderRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    expanded: bool,
    on_toggle: Message,
    content: Option<Element<'a, Message>>,
}

impl<'a, Message> ExpanderRow<'a, Message> {
    pub fn new(on_toggle: Message) -> Self {
        Self {
            title: "",
            description: "",
            expanded: false,
            on_toggle,
            content: None,
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

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn content(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.content = Some(content.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        let expanded = expander.expanded;
        let header = button(
            row![
                column![
                    text(expander.title).size(18).style(text::base),
                    text(expander.description).size(16).style(style::muted_text),
                ]
                .width(Fill)
                .spacing(4),
                svg(crate::icons::get("down_caret"))
                    .width(20)
                    .height(20)
                    .content_fit(ContentFit::Contain)
                    .rotation(if expanded { std::f32::consts::PI } else { 0.0 }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([18, 24])
        .on_press(expander.on_toggle)
        .style(header_style);

        let mut contents = column![header].width(Fill);

        if expanded && let Some(content) = expander.content {
            contents = contents.push(container(content).width(Fill).padding(18));
        }

        RowSurface::new(container(contents).width(Fill).clip(true))
            .raised(expanded)
            .into()
    }
}

fn header_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Pressed).then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: theme.palette().text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::text;

    use super::ExpanderRow;

    #[test]
    fn content_is_optional() {
        let empty = ExpanderRow::<()>::new(());
        let populated = ExpanderRow::new(()).content(text("child"));

        assert!(empty.content.is_none());
        assert!(populated.content.is_some());
    }
}
