use iced::{
    Alignment, Background, Border, Element, Fill, Theme,
    widget::{button, column, row, rule, text},
};

use super::{
    pressable::{Pressable, Status},
    text::TextExt as _,
};

pub struct Tab<'a, T> {
    value: T,
    label: &'a str,
    enabled: bool,
}

impl<'a, T> Tab<'a, T> {
    pub fn new(value: T, label: &'a str) -> Self {
        Self {
            value,
            label,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub struct Tabs<'a, T, Message> {
    tabs: Vec<Tab<'a, T>>,
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
}

impl<'a, T, Message> Tabs<'a, T, Message> {
    pub fn new(
        tabs: impl IntoIterator<Item = Tab<'a, T>>,
        selected: Option<T>,
        on_select: impl Fn(T) -> Message + 'a,
    ) -> Self {
        Self {
            tabs: tabs.into_iter().collect(),
            selected,
            on_select: Box::new(on_select),
        }
    }
}

impl<'a, T, Message> From<Tabs<'a, T, Message>> for Element<'a, Message>
where
    T: PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(tabs: Tabs<'a, T, Message>) -> Self {
        let Tabs {
            tabs,
            selected,
            on_select,
        } = tabs;
        let children = tabs.into_iter().map(|tab| {
            let selected = selected.as_ref() == Some(&tab.value);
            let message = tab.enabled.then(|| on_select(tab.value));

            Pressable::new(
                column![
                    text(tab.label).label(),
                    rule::horizontal(3).style(move |theme: &Theme| rule::Style {
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
            .on_press_maybe(message)
            .style(move |theme, status| tab_style(theme, status, selected))
            .into()
        });

        row(children).width(Fill).into()
    }
}

fn tab_style(theme: &Theme, status: Status, selected: bool) -> button::Style {
    button::Style {
        background: matches!(status, Status::Focused).then_some(Background::Color(
            theme.extended_palette().background.strong.color,
        )),
        text_color: if selected
            || matches!(status, Status::Hovered | Status::Pressed | Status::Focused)
        {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}
