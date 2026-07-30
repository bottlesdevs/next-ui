use iced::{
    Alignment, ContentFit, Element, Fill,
    widget::{Space, button, column, row, svg, text},
};

use crate::icons;

use super::style;

pub struct PickerRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    on_press: Option<Message>,
}

impl<Message> PickerRow<'_, Message> {
    pub fn new() -> Self {
        Self {
            title: "",
            description: "",
            on_press: None,
        }
    }
}

impl<'a, Message> PickerRow<'a, Message> {
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }
}

impl<Message> Default for PickerRow<'_, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<PickerRow<'a, Message>> for Element<'a, Message> {
    fn from(picker: PickerRow<'a, Message>) -> Self {
        let labels = column![
            text(picker.title).size(18).style(text::base),
            text(picker.description).size(16).style(style::muted_text),
        ]
        .spacing(4);

        let row = button(
            row![
                labels,
                Space::new().width(Fill),
                svg(icons::get("folder"))
                    .width(25)
                    .height(20)
                    .content_fit(ContentFit::Contain),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([18, 24])
        .style(style::action);

        match picker.on_press {
            Some(on_press) => row.on_press(on_press).into(),
            None => row.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PickerRow;

    #[test]
    fn click_action_is_optional() {
        assert!(PickerRow::<()>::new().on_press.is_none());
        assert!(PickerRow::new().on_press(()).on_press.is_some());
    }
}
