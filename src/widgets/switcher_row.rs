use iced::{Element, widget::text::IntoFragment};

use super::{list_row::ListRow, switcher::Switcher};

pub struct SwitcherRow<'a, Message> {
    row: ListRow<'a, Message>,
    value: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message: 'a> SwitcherRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, value: bool) -> Self {
        Self {
            row: ListRow::labeled(title).subtitle(""),
            value,
            on_toggle: None,
        }
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.row = self.row.subtitle(description);
        self
    }

    /// Places `accessory` before the switch, e.g. a [`Popover`] that picks
    /// which release the switch turns on.
    ///
    /// [`Popover`]: super::popover::Popover
    pub fn accessory(mut self, accessory: impl Into<Element<'a, Message>>) -> Self {
        self.row = self.row.trailing(accessory);
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
        switcher
            .row
            .trailing(Switcher::new(switcher.value).on_toggle_maybe(switcher.on_toggle))
    }
}
