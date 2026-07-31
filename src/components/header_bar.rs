use iced::{
    Background, Border, Center, Element, Fill, Length, Task, Theme,
    widget::{Row, button, container, mouse_area, row, svg},
    window,
};

use crate::{icons, theme};

const HEIGHT: f32 = 64.0;
const SPACING: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Drag,
    Close,
}

impl Action {
    pub fn task<Message: Send + 'static>(self) -> Task<Message> {
        match self {
            Self::Drag => window::latest().and_then(window::drag),
            Self::Close => window::latest().and_then(window::close),
        }
    }
}

pub struct HeaderBar<'a, Message> {
    start: Vec<Element<'a, Message>>,
    middle: Vec<Element<'a, Message>>,
    end: Vec<Element<'a, Message>>,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
}

impl<'a, Message> HeaderBar<'a, Message> {
    pub fn new(on_action: impl Fn(Action) -> Message + 'a) -> Self {
        Self {
            start: Vec::new(),
            middle: Vec::new(),
            end: Vec::new(),
            on_action: Box::new(on_action),
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
}

impl<'a, Message: Clone + 'a> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(header: HeaderBar<'a, Message>) -> Self {
        let HeaderBar {
            mut start,
            middle,
            mut end,
            on_action,
        } = header;
        let close = window_control(on_action(Action::Close));

        if cfg!(target_os = "macos") {
            start.insert(0, close);
        } else {
            end.push(close);
        }

        let content = row![
            container(section(start)).align_left(Fill),
            container(section(middle)).center_x(Length::Shrink),
            container(section(end)).align_right(Fill),
        ]
        .height(Fill)
        .align_y(Center);

        mouse_area(
            container(content)
                .width(Fill)
                .height(HEIGHT)
                .padding([6, 24])
                .style(style),
        )
        .on_press(on_action(Action::Drag))
        .into()
    }
}

fn section<'a, Message: 'a>(children: Vec<Element<'a, Message>>) -> Row<'a, Message> {
    Row::with_children(children)
        .spacing(SPACING)
        .align_y(Center)
}

fn window_control<'a, Message: Clone + 'a>(message: Message) -> Element<'a, Message> {
    button(container(svg(icons::get("cross")).width(16).height(16)).center(40))
        .width(40)
        .height(40)
        .padding(0)
        .on_press(message)
        .style(|theme: &Theme, status| {
            let colors = theme.extended_palette();

            button::Style {
                background: match status {
                    button::Status::Pressed => {
                        Some(Background::Color(colors.background.stronger.color))
                    }
                    _ => None,
                },
                text_color: colors.secondary.weak.text,
                border: Border::default().rounded(8),
                ..button::Style::default()
            }
        })
        .into()
}

fn style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::WINDOW)),
        ..container::Style::default()
    }
}
