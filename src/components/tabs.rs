use iced::{Element, Fill, widget::row};

use super::tab::Tab;

pub struct Tabs<'a, Message> {
    labels: &'a [&'a str],
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a, Message> Tabs<'a, Message> {
    pub fn new(labels: &'a [&'a str], on_select: impl Fn(usize) -> Message + 'a) -> Self {
        Self {
            labels,
            selected: 0,
            on_select: Box::new(on_select),
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Tabs<'a, Message>> for Element<'a, Message> {
    fn from(tabs: Tabs<'a, Message>) -> Self {
        let Tabs {
            labels,
            selected,
            on_select,
        } = tabs;

        row(labels.iter().enumerate().map(move |(index, label)| {
            Tab::new(label, on_select(index))
                .selected(selected == index)
                .into()
        }))
        .width(Fill)
        .into()
    }
}
