use iced::Element;

use super::{
    list_row::{ListRow, labels},
    switcher::Switcher,
};

pub struct SwitcherRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    value: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> SwitcherRow<'a, Message> {
    pub fn new(title: &'a str, value: bool) -> Self {
        Self {
            title,
            description: "",
            value,
            on_toggle: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn on_toggle_maybe(mut self, on_toggle: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.on_toggle = on_toggle.map(|on_toggle| Box::new(on_toggle) as _);
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
        let enabled = switcher.on_toggle.is_some();

        ListRow::new(labels(switcher.title, switcher.description))
            .trailing(Switcher::new(switcher.value).on_toggle_maybe(switcher.on_toggle))
            .enabled(enabled)
    }
}
