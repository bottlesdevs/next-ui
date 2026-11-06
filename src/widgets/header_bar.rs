use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    widget::{Row, container, mouse_area, row},
};

use crate::{icons::Icon, theme};

use super::{
    button::{Button, ButtonKind},
    header, spacing,
};

/// Builds a header-sized icon button; the caller supplies its action.
pub fn header_button<Message>(label: &str, icon: Icon) -> Button<'_, Message> {
    Button::icon_only(label, icon)
        .diameter(header::CONTROL)
        .icon_size(header::ICON)
        .kind(ButtonKind::Transparent)
}

pub struct HeaderBar<'a, Message> {
    start: Vec<Element<'a, Message>>,
    middle: Vec<Element<'a, Message>>,
    end: Vec<Element<'a, Message>>,
    transparent: bool,
    on_drag: Message,
}

impl<'a, Message> HeaderBar<'a, Message> {
    pub fn new(on_drag: Message) -> Self {
        Self {
            start: Vec::new(),
            middle: Vec::new(),
            end: Vec::new(),
            transparent: false,
            on_drag,
        }
    }

    pub fn start(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.start.push(content.into());
        self
    }

    pub fn middle(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.middle.push(content.into());
        self
    }

    pub fn end(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.end.push(content.into());
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
}

impl<'a, Message: Clone + 'a> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(header: HeaderBar<'a, Message>) -> Self {
        let HeaderBar {
            start,
            middle,
            end,
            transparent,
            on_drag,
        } = header;

        let content = row![
            container(section(start))
                .align_left(Length::Shrink)
                .height(Fill)
                .align_y(Center),
            container(section(middle))
                .center_x(Fill)
                .height(Fill)
                .align_y(Center),
            container(section(end))
                .align_right(Length::Shrink)
                .height(Fill)
                .align_y(Center),
        ]
        .height(Fill)
        .align_y(Center);

        let mut content = container(content)
            .width(Fill)
            .height(header::HEIGHT)
            .padding([0.0, header::INSET]);
        if !transparent {
            content = content.style(style);
        }

        mouse_area(content).on_press(on_drag).into()
    }
}

fn section<'a, Message: 'a>(children: Vec<Element<'a, Message>>) -> Row<'a, Message> {
    Row::with_children(children)
        .spacing(spacing::SM)
        .align_y(Center)
}

fn style(current_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::colors(current_theme).window)),
        border: Border::default().rounded(iced::border::top(6)),
        ..container::Style::default()
    }
}
