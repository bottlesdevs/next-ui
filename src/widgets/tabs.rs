use iced::{
    Alignment, Background, Border, Element, Fill, Length, Theme,
    widget::{Row, Space, button, container, stack, text},
};

use super::{
    focusable_action::{ActionState, FocusableAction},
    spacing,
    text::TextExt as _,
};
use crate::theme;

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
    height: Length,
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
            height: Length::Shrink,
        }
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
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
            height,
        } = tabs;
        let children: Vec<Element<'a, Message>> = tabs
            .into_iter()
            .map(|tab| {
                let selected = selected.as_ref() == Some(&tab.value);
                let message = tab.enabled.then(|| on_select(tab.value));

                let indicator_height = 3f32;
                let indicator = container(Space::new().height(indicator_height))
                    .width(Fill)
                    .style(move |theme: &Theme| {
                        container::Style::default().background(Background::Color(if selected {
                            theme::colors(theme).accent
                        } else {
                            iced::Color::TRANSPARENT
                        }))
                    });

                let label = FocusableAction::new(
                    text(tab.label)
                        .label()
                        .height(height)
                        .align_y(Alignment::Center),
                )
                .height(height)
                .padding(spacing::SM)
                .selected(selected)
                .on_press_maybe(message)
                .style(tab_style);

                stack![label, container(indicator).align_bottom(Fill)]
                    .height(height)
                    .into()
            })
            .collect();

        let content: Element<'a, Message> = container(
            Row::with_children(children)
                .spacing(spacing::LG)
                .height(height),
        )
        .height(height)
        .center_x(Fill)
        .into();

        content
    }
}

fn tab_style(theme: &Theme, state: ActionState) -> button::Style {
    let colors = theme::colors(theme);

    button::Style {
        background: (state.focused && !state.hovered && !state.pressed)
            .then_some(Background::Color(colors.hover)),
        text_color: if state.selected || state.hovered || state.pressed || state.focused {
            colors.text
        } else {
            colors.muted
        },
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}
