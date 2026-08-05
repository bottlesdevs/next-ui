use iced::{Element, Task, window as iced_window};

#[cfg(target_os = "linux")]
use iced::{
    Fill,
    alignment::{Horizontal, Vertical},
    mouse,
    widget::{Space, container, mouse_area, stack},
    window::Direction,
};

use crate::theme;

#[cfg(not(target_os = "macos"))]
const RESIZE_EDGE: f32 = 6.0;
#[cfg(not(target_os = "macos"))]
const RESIZE_CORNER: f32 = 12.0;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Drag,
    Resize(iced_window::Direction),
    Close,
}

impl Action {
    pub fn task<Message: Send + 'static>(self) -> Task<Message> {
        match self {
            Self::Drag => iced_window::latest().and_then(iced_window::drag),
            Self::Resize(direction) => {
                iced_window::latest().and_then(move |id| iced_window::drag_resize(id, direction))
            }
            Self::Close => iced_window::latest().and_then(iced_window::close),
        }
    }
}

pub struct WindowFrame<'a, Message> {
    content: Element<'a, Message>,
    #[cfg(not(target_os = "macos"))]
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
}

impl<'a, Message> WindowFrame<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        on_action: impl Fn(Action) -> Message + 'a,
    ) -> Self {
        #[cfg(target_os = "macos")]
        let _ = on_action;

        Self {
            content: content.into(),
            #[cfg(not(target_os = "macos"))]
            on_action: Box::new(on_action),
        }
    }
}

impl<'a, Message: Clone + 'a> From<WindowFrame<'a, Message>> for Element<'a, Message> {
    fn from(frame: WindowFrame<'a, Message>) -> Self {
        let content: Element<'a, Message> = container(frame.content)
            .width(Fill)
            .height(Fill)
            .padding(1)
            .style(theme::window)
            .clip(true)
            .into();

        #[cfg(target_os = "macos")]
        return content;

        #[cfg(not(target_os = "macos"))]
        {
            use iced::window::Direction;

            let mut layers = stack![content].width(Fill).height(Fill).clip(true);

            for direction in [
                Direction::North,
                Direction::South,
                Direction::East,
                Direction::West,
                Direction::NorthEast,
                Direction::NorthWest,
                Direction::SouthEast,
                Direction::SouthWest,
            ] {
                layers = layers.push(resize_edge(
                    direction,
                    (frame.on_action)(Action::Resize(direction)),
                ))
            }
            layers.into()
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn resize_edge<'a, Message: Clone + 'a>(
    direction: Direction,
    message: Message,
) -> Element<'a, Message> {
    let (width, height, interaction, horizontal, vertical) = match direction {
        Direction::North => (
            Fill,
            RESIZE_EDGE.into(),
            mouse::Interaction::ResizingVertically,
            Horizontal::Left,
            Vertical::Top,
        ),
        Direction::South => (
            Fill,
            RESIZE_EDGE.into(),
            mouse::Interaction::ResizingVertically,
            Horizontal::Left,
            Vertical::Bottom,
        ),
        Direction::East => (
            RESIZE_EDGE.into(),
            Fill,
            mouse::Interaction::ResizingHorizontally,
            Horizontal::Right,
            Vertical::Top,
        ),
        Direction::West => (
            RESIZE_EDGE.into(),
            Fill,
            mouse::Interaction::ResizingHorizontally,
            Horizontal::Left,
            Vertical::Top,
        ),
        Direction::NorthEast => (
            RESIZE_CORNER.into(),
            RESIZE_CORNER.into(),
            mouse::Interaction::ResizingDiagonallyUp,
            Horizontal::Right,
            Vertical::Top,
        ),
        Direction::NorthWest => (
            RESIZE_CORNER.into(),
            RESIZE_CORNER.into(),
            mouse::Interaction::ResizingDiagonallyDown,
            Horizontal::Left,
            Vertical::Top,
        ),
        Direction::SouthEast => (
            RESIZE_CORNER.into(),
            RESIZE_CORNER.into(),
            mouse::Interaction::ResizingDiagonallyUp,
            Horizontal::Right,
            Vertical::Bottom,
        ),
        Direction::SouthWest => (
            RESIZE_CORNER.into(),
            RESIZE_CORNER.into(),
            mouse::Interaction::ResizingDiagonallyDown,
            Horizontal::Left,
            Vertical::Bottom,
        ),
    };

    container(
        mouse_area(Space::new().width(width).height(height))
            .on_press(message)
            .interaction(interaction),
    )
    .width(Fill)
    .height(Fill)
    .align_x(horizontal)
    .align_y(vertical)
    .into()
}
