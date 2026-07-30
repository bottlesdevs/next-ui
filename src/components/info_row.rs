use iced::{
    Alignment, ContentFit, Element, Fill,
    widget::{container, row, svg, text},
};

use super::style;

pub struct InfoRow<'a> {
    title: &'a str,
    description: &'a str,
    icon: Option<svg::Handle>,
}

impl InfoRow<'_> {
    pub fn new() -> Self {
        Self {
            title: "",
            description: "",
            icon: None,
        }
    }
}

impl<'a> InfoRow<'a> {
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: impl Into<svg::Handle>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl Default for InfoRow<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<InfoRow<'a>> for Element<'a, Message> {
    fn from(info: InfoRow<'a>) -> Self {
        let labels = iced::widget::column![
            text(info.title).size(18).style(text::base),
            text(info.description).size(16).style(style::muted_text),
        ]
        .spacing(4);

        let mut content = row![].spacing(24).align_y(Alignment::Center);

        if let Some(icon) = info.icon {
            content = content.push(
                svg(icon)
                    .width(24)
                    .height(24)
                    .content_fit(ContentFit::Contain),
            );
        }

        container(content.push(labels))
            .width(Fill)
            .padding([18, 24])
            .style(style::surface)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use crate::icons;

    use super::InfoRow;

    #[test]
    fn icon_is_optional() {
        assert!(InfoRow::new().icon.is_none());
        assert!(InfoRow::new().icon(icons::get("timer")).icon.is_some());
    }
}
