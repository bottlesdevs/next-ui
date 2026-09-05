use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    widget::{Row, container, mouse_area, row},
};

use crate::theme;

use super::spacing;

const HEIGHT: f32 = 64.0;

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
                .align_left(Fill)
                .align_bottom(Fill)
                .padding(iced::padding::bottom(spacing::MD)),
            container(section(middle))
                .center_x(Length::Shrink)
                .align_bottom(Fill),
            container(section(end))
                .align_right(Fill)
                .align_bottom(Fill)
                .padding(iced::padding::bottom(spacing::MD)),
        ]
        .height(Fill)
        .align_y(Center);

        let mut content = container(content)
            .width(Fill)
            .height(HEIGHT)
            .padding([0.0, spacing::SM]);
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
