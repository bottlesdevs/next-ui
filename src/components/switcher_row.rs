use iced::Element;

use super::{
    list_row::{ListRow, labels},
    switcher::Switcher,
};

pub struct SwitcherRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    value: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message> SwitcherRow<'a, Message> {
    pub fn new(value: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        Self {
            title: "",
            description: "",
            value,
            on_toggle: Box::new(on_toggle),
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
}

impl<'a, Message: Clone + 'a> From<SwitcherRow<'a, Message>> for Element<'a, Message> {
    fn from(switcher: SwitcherRow<'a, Message>) -> Self {
        ListRow::from(switcher).into()
    }
}

impl<'a, Message: Clone + 'a> From<SwitcherRow<'a, Message>> for ListRow<'a, Message> {
    fn from(switcher: SwitcherRow<'a, Message>) -> Self {
        ListRow::new(labels(switcher.title, switcher.description))
            .trailing(Switcher::new(switcher.value, switcher.on_toggle))
    }
}
