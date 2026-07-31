use iced::{
    Alignment, Element, Fill,
    widget::{button, column, text},
};

use crate::icons::Icon;

use super::{list_row::ListRow, style, text::TextExt as _};

pub struct CycleRow<'a, Message> {
    title: &'a str,
    value: &'a str,
    previous: Option<Message>,
    next: Option<Message>,
}

impl<Message> CycleRow<'_, Message> {
    pub fn new() -> Self {
        Self {
            title: "",
            value: "",
            previous: None,
            next: None,
        }
    }
}

impl<'a, Message> CycleRow<'a, Message> {
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub fn on_previous(mut self, previous: Message) -> Self {
        self.previous = Some(previous);
        self
    }

    pub fn on_next(mut self, next: Message) -> Self {
        self.next = Some(next);
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
        let previous = button(Icon::Arrow.view())
            .padding(8)
            .style(style::action)
            .on_press_maybe(cycle.previous);

        let next = button(Icon::Arrow.rotated(std::f32::consts::PI))
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

        ListRow::new(labels).leading(previous).trailing(next)
    }
}

#[cfg(test)]
mod tests {
    use super::CycleRow;

    #[test]
    fn each_direction_can_be_disabled() {
        let cycle = CycleRow::new().on_next(());

        assert!(cycle.previous.is_none());
        assert!(cycle.next.is_some());
    }
}
