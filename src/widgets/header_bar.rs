use iced::{
    Background, Border, Center, Element, Fill, Length, Theme,
    widget::{Row, container, mouse_area, row},
};

use crate::{icons::Icon, theme};

use super::{
    button::{Button, ButtonKind},
    spacing,
    window_frame::Action,
};

const HEIGHT: f32 = 64.0;

pub struct HeaderBar<'a, Message> {
    start: Vec<Element<'a, Message>>,
    middle: Vec<Element<'a, Message>>,
    end: Vec<Element<'a, Message>>,
    show_window_controls: bool,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
}

impl<'a, Message> HeaderBar<'a, Message> {
    pub fn new(on_action: impl Fn(Action) -> Message + 'a) -> Self {
        Self {
            start: Vec::new(),
            middle: Vec::new(),
            end: Vec::new(),
            show_window_controls: true,
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

    pub fn show_window_controls(mut self, show_window_controls: bool) -> Self {
        self.show_window_controls = show_window_controls;
        self
    }
}

impl<'a, Message: Clone + 'a> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(header: HeaderBar<'a, Message>) -> Self {
        let HeaderBar {
            mut start,
            middle,
            mut end,
            show_window_controls,
            on_action,
        } = header;

        if show_window_controls {
            let close = window_control(on_action(Action::Close));

            if cfg!(target_os = "macos") {
                start.insert(0, close);
            } else {
                end.push(close);
            }
        }

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

        mouse_area(
            container(content)
                .width(Fill)
                .height(HEIGHT)
                .padding([0.0, spacing::SM])
                .style(style),
        )
        .on_press(on_action(Action::Drag))
        .into()
    }
}

fn section<'a, Message: 'a>(children: Vec<Element<'a, Message>>) -> Row<'a, Message> {
    Row::with_children(children)
        .spacing(spacing::SM)
        .align_y(Center)
}

fn window_control<'a, Message: Clone + 'a>(message: Message) -> Element<'a, Message> {
    Button::icon_only("Close window", Icon::Cross)
        .diameter(32.0)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent)
        .on_press(message)
        .into()
}

fn style(current_theme: &Theme) -> container::Style {
    let bottles_theme = theme::BottlesTheme::from(current_theme);

    container::Style {
        background: Some(Background::Color(bottles_theme.window)),
        border: Border::default().rounded(iced::border::top(11)),
        ..container::Style::default()
    }
}
