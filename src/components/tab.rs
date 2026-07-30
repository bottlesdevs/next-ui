use iced::{
    Alignment, Element, Fill,
    widget::{button, column, rule, text},
};

use super::style;

pub struct Tab<'a, Message> {
    label: &'a str,
    selected: bool,
    on_press: Message,
}

impl<'a, Message> Tab<'a, Message> {
    pub fn new(label: &'a str, on_press: Message) -> Self {
        Self {
            label,
            selected: false,
            on_press,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Tab<'a, Message>> for Element<'a, Message> {
    fn from(tab: Tab<'a, Message>) -> Self {
        let Tab {
            label,
            selected,
            on_press,
        } = tab;

        button(
            column![
                text(label).size(18),
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
}
