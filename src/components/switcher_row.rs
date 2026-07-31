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
    enabled: bool,
}

impl<'a, Message> SwitcherRow<'a, Message> {
    pub fn new(title: &'a str, value: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        Self {
            title,
            description: "",
            value,
            on_toggle: Box::new(on_toggle),
            enabled: true,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
            .trailing(
                Switcher::new(switcher.value)
                    .on_toggle_maybe(switcher.enabled.then_some(switcher.on_toggle)),
            )
            .enabled(switcher.enabled)
    }
}
