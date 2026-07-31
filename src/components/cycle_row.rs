use iced::{
    Alignment, Element, Fill,
    widget::{column, text},
};

use crate::icons::Icon;

use super::{list_row::ListRow, pressable::Pressable, style, text::TextExt as _};

pub struct CycleRow<'a, Message> {
    title: &'a str,
    value: &'a str,
    previous: Option<Message>,
    next: Option<Message>,
}

impl<'a, Message> CycleRow<'a, Message> {
    pub fn new(title: &'a str, value: &'a str) -> Self {
        Self {
            title,
            value,
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
        let previous = Pressable::new(Icon::Arrow.view())
            .padding(8)
            .style(style::action)
            .on_press_maybe(cycle.previous);

        let next = Pressable::new(Icon::Arrow.rotated(std::f32::consts::PI))
            .padding(8)
            .style(style::action)
            .on_press_maybe(cycle.next);

        let labels = column![
            text(cycle.title).label(),
            text(cycle.value).detail().muted(),
        ]
        .width(Fill)
        .align_x(Alignment::Center)
        .spacing(4);

        ListRow::new(labels)
            .leading(previous)
            .trailing(next)
            .enabled(enabled)
    }
}
