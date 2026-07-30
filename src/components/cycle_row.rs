use iced::{
    Alignment, ContentFit, Element, Fill,
    widget::{button, column, container, row, svg, text},
};

use crate::icons;

use super::{row_surface::RowSurface, style};

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

impl<Message> Default for CycleRow<'_, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<CycleRow<'a, Message>> for Element<'a, Message> {
    fn from(cycle: CycleRow<'a, Message>) -> Self {
        let previous = button(
            svg(icons::get("arrow"))
                .width(24)
                .height(24)
                .content_fit(ContentFit::Contain),
        )
        .padding(8)
        .style(style::action);
        let previous: Element<'a, Message> = match cycle.previous {
            Some(message) => previous.on_press(message).into(),
            None => previous.into(),
        };

        let next = button(
            svg(icons::get("arrow"))
                .width(24)
                .height(24)
                .content_fit(ContentFit::Contain)
                .rotation(std::f32::consts::PI),
        )
        .padding(8)
        .style(style::action);
        let next: Element<'a, Message> = match cycle.next {
            Some(message) => next.on_press(message).into(),
            None => next.into(),
        };

        let labels = column![
            text(cycle.title).size(18).style(text::base),
            text(cycle.value).size(16).style(style::muted_text),
        ]
        .width(Fill)
        .align_x(Alignment::Center)
        .spacing(4);

        RowSurface::new(
            container(
                row![previous, labels, next]
                    .spacing(16)
                    .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([18, 24]),
        )
        .into()
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
