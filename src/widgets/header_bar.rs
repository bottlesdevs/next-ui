use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    widget::{Row, container, row},
};

use crate::{icons::Icon, theme};

use super::{
    button::{Button, ButtonKind},
    header, radius, spacing,
    window_handle::WindowHandle,
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
}

impl<Message> Default for HeaderBar<'_, Message> {
    fn default() -> Self {
        Self {
            start: Vec::new(),
            middle: Vec::new(),
            end: Vec::new(),
            transparent: false,
        }
    }
}

impl<'a, Message> HeaderBar<'a, Message> {
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

impl<'a, Message: 'a> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(header: HeaderBar<'a, Message>) -> Self {
        let HeaderBar {
            start,
            middle,
            end,
            transparent,
        } = header;

        // The sides share what the middle leaves equally, so the middle sits
        // at the centre of the bar whatever either side holds.
        let content = row![
            container(section(start))
                .align_left(Fill)
                .height(Fill)
                .align_y(Center)
                .clip(true),
            container(section(middle))
                .width(Length::Shrink)
                .height(Fill)
                .align_y(Center),
            container(section(end))
                .align_right(Fill)
                .height(Fill)
                .align_y(Center)
                .clip(true),
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

        WindowHandle::new(content).into()
    }
}

fn section<'a, Message: 'a>(children: Vec<Element<'a, Message>>) -> Row<'a, Message> {
    Row::with_children(children)
        .spacing(spacing::SM)
        .align_y(Center)
}

fn style(current_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::colors(current_theme).chrome)),
        border: Border::default().rounded(iced::border::top(radius::ROW)),
        ..container::Style::default()
    }
}
