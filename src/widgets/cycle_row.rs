use iced::{
    Element,
    widget::text::{Fragment, IntoFragment},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    list_row::ListRow,
    spacing,
};

/// A row that steps through values with arrows on either side of its
/// centred labels.
pub struct CycleRow<'a, Message> {
    title: Fragment<'a>,
    value: Fragment<'a>,
    previous: Option<Message>,
    next: Option<Message>,
}

impl<'a, Message> CycleRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            value: value.into_fragment(),
            previous: None,
            next: None,
        }
    }

    pub fn on_previous(mut self, previous: Message) -> Self {
        self.previous = Some(previous);
        self
    }

    pub fn on_next(mut self, next: Message) -> Self {
        self.next = Some(next);
        self
    }

    pub fn on_previous_maybe(mut self, previous: Option<Message>) -> Self {
        self.previous = previous;
        self
    }

    pub fn on_next_maybe(mut self, next: Option<Message>) -> Self {
        self.next = next;
        self
    }
}

impl<'a, Message: Clone + 'a> From<CycleRow<'a, Message>> for Element<'a, Message> {
    fn from(cycle: CycleRow<'a, Message>) -> Self {
        ListRow::from(cycle).into()
    }
}

impl<'a, Message: Clone + 'a> From<CycleRow<'a, Message>> for ListRow<'a, Message> {
    fn from(cycle: CycleRow<'a, Message>) -> Self {
        let enabled = cycle.previous.is_some() || cycle.next.is_some();
        let previous = Button::icon_only("Previous value", Icon::Arrow)
            .kind(ButtonKind::Transparent)
            .diameter(32.0)
            .on_press_maybe(cycle.previous);
        let next = Button::icon_only("Next value", Icon::Arrow)
            .kind(ButtonKind::Transparent)
            .diameter(32.0)
            .icon_rotation(std::f32::consts::PI)
            .on_press_maybe(cycle.next);

        ListRow::labeled(cycle.title)
            .subtitle(cycle.value)
            .centered()
            .dimmed(!enabled)
            .leading(previous)
            .trailing(next)
            .padding([spacing::MD, spacing::LG])
    }
}
