use iced::{ContentFit, Element, widget::svg};

use crate::icons::Icon;

use super::list_row::{ListRow, labels};

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

impl<'a, Message: Clone + 'a> From<PickerRow<'a, Message>> for Element<'a, Message> {
    fn from(picker: PickerRow<'a, Message>) -> Self {
        ListRow::from(picker).into()
    }
}

impl<'a, Message: Clone + 'a> From<PickerRow<'a, Message>> for ListRow<'a, Message> {
    fn from(picker: PickerRow<'a, Message>) -> Self {
        let row = ListRow::new(labels(picker.title, picker.description)).trailing(
            svg(Icon::Folder.handle())
                .width(25)
                .height(20)
                .content_fit(ContentFit::Contain),
        );
        match picker.on_press {
            Some(on_press) => row.on_press(on_press),
            None => row,
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
