use iced::{
    Alignment, Element, Fill, Theme,
    widget::{column, container, text},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    list_row::{self, ListRow},
    spacing,
    text::TextExt as _,
};

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
        cycle.lower().into()
    }
}

impl<'a, Message: Clone + 'a> From<CycleRow<'a, Message>> for ListRow<'a, Message> {
    fn from(cycle: CycleRow<'a, Message>) -> Self {
        cycle.lower()
    }
}

impl<'a, Message: Clone + 'a> CycleRow<'a, Message> {
    fn lower(self) -> ListRow<'a, Message> {
        let previous = self.previous;
        let next = self.next;
        let enabled = previous.is_some() || next.is_some();
        let previous = Button::icon_only("Previous value", Icon::Arrow)
            .kind(ButtonKind::Transparent)
            .diameter(32.0)
            .on_press_maybe(previous);

        let next = Button::icon_only("Next value", Icon::Arrow)
            .kind(ButtonKind::Transparent)
            .diameter(32.0)
            .icon_rotation(std::f32::consts::PI)
            .on_press_maybe(next);

        let labels = container(
            column![
                text(self.title).label().medium(),
                text(self.value).size(list_row::BODY_SIZE).muted(),
            ]
            .width(Fill)
            .align_x(Alignment::Center)
            .spacing(spacing::XS),
        )
        .style(move |theme: &Theme| {
            container::Style::default().color(if enabled {
                crate::theme::colors(theme).text
            } else {
                crate::theme::colors(theme).muted
            })
        });

        ListRow::new(labels)
            .leading(previous)
            .trailing(next)
            .padding([spacing::MD, spacing::LG])
    }
}
