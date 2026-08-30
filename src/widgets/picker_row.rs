use iced::Element;

use crate::icons::Icon;

use super::list_row::{ListRow, labels};

pub struct PickerRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    on_press: Option<Message>,
}

impl<'a, Message> PickerRow<'a, Message> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            description: "",
            on_press: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
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
        let row = ListRow::new(labels(picker.title, picker.description))
            .trailing(Icon::Folder.view().width(25).height(20));
        match picker.on_press {
            Some(on_press) => row.on_press(on_press),
            None => row.enabled(false),
        }
    }
}
