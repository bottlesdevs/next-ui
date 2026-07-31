use iced::{
    Alignment, Element, Fill,
    widget::{button, column, row, rule, text},
};

use super::{style, text::TextExt as _};

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

        row(labels
            .iter()
            .enumerate()
            .map(move |(index, label)| tab(label, selected == index, on_select(index))))
        .width(Fill)
        .into()
    }
}

fn tab<'a, Message: Clone + 'a>(
    label: &'a str,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        column![
            text(label).label(),
            rule::horizontal(3).style(move |theme: &iced::Theme| rule::Style {
                color: if selected {
                    theme.palette().primary
                } else {
                    iced::Color::TRANSPARENT
                },
                ..rule::default(theme)
            }),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .width(Fill)
    .padding([8, 16])
    .on_press(on_press)
    .style(move |theme, status| {
        let mut tab = style::tab(theme, status);
        tab.text_color = match status {
            button::Status::Hovered | button::Status::Pressed => theme.palette().text,
            _ if selected => theme.palette().text,
            _ => theme.extended_palette().secondary.weak.text,
        };
        tab
    })
    .into()
}
