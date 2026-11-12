use iced::{
    Element,
    widget::text::{Fragment, IntoFragment},
};

use super::{
    list_row::{ListRow, labels},
    switcher::Switcher,
};

pub struct SwitcherRow<'a, Message> {
    title: &'a str,
    description: Fragment<'a>,
    value: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    accessory: Option<Element<'a, Message>>,
}

impl<'a, Message> SwitcherRow<'a, Message> {
    pub fn new(title: &'a str, value: bool) -> Self {
        Self {
            title,
            description: Fragment::Borrowed(""),
            value,
            on_toggle: None,
            accessory: None,
        }
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = description.into_fragment();
        self
    }

    /// Places `accessory` before the switch, e.g. a [`Popover`] that picks
    /// which release the switch turns on.
    ///
    /// [`Popover`]: super::popover::Popover
    pub fn accessory(mut self, accessory: impl Into<Element<'a, Message>>) -> Self {
        self.accessory = Some(accessory.into());
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
        let mut row = ListRow::new(labels(switcher.title, switcher.description));
        if let Some(accessory) = switcher.accessory {
            row = row.trailing(accessory);
        }
        row.trailing(Switcher::new(switcher.value).on_toggle_maybe(switcher.on_toggle))
    }
}
