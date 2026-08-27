use iced::{Element, Fill, Task, widget::container, window};

use iced::{
    alignment::{Horizontal, Vertical},
    mouse,
    widget::{Space, mouse_area, stack},
    window::Direction,
};

use crate::{
    icons::Icon,
    theme,
    widgets::button::{Button, ButtonKind},
};

const RESIZE_EDGE: f32 = 6.0;
const RESIZE_CORNER: f32 = 12.0;
const PANEL_INSET: [f32; 2] = [6.0, 8.0];
const WINDOW_CONTROL_INSET: [f32; 2] = [22.0, 20.0];
pub(crate) const WINDOW_CONTROL_SIZE: f32 = 32.0;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Drag,
    Resize(window::Direction),
    RequestClose,
}

impl Action {
    /// Returns the direct window operation for this action.
    ///
    /// A close request returns `None` so the application can shut down its
    /// services before exiting.
    pub fn task<Message: Send + 'static>(self) -> Option<Task<Message>> {
        Some(match self {
            Self::Drag => window::latest().and_then(window::drag),
            Self::Resize(direction) => {
                window::latest().and_then(move |id| window::drag_resize(id, direction))
            }
            Self::RequestClose => return None,
        })
    }
}

pub struct WindowFrame<'a, Message> {
    content: Element<'a, Message>,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
}

impl<'a, Message> WindowFrame<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        on_action: impl Fn(Action) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_action: Box::new(on_action),
        }
    }
}

impl<'a, Message: Clone + 'a> From<WindowFrame<'a, Message>> for Element<'a, Message> {
    fn from(frame: WindowFrame<'a, Message>) -> Self {
        let content: Element<'a, Message> = container(frame.content)
            .width(Fill)
            .height(Fill)
            .padding(PANEL_INSET)
            .style(|current_theme| theme::window(&theme::BottlesTheme::from(current_theme)))
            .clip(true)
            .into();

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

        let close = Button::icon_only("Close window", Icon::Cross)
            .diameter(WINDOW_CONTROL_SIZE)
            .icon_size(16.0)
            .kind(ButtonKind::Transparent)
            .on_press((frame.on_action)(Action::RequestClose));
        let close = container(close)
            .width(Fill)
            .height(Fill)
            .align_x(if cfg!(target_os = "macos") {
                Horizontal::Left
            } else {
                Horizontal::Right
            })
            .align_y(Vertical::Top)
            .padding(WINDOW_CONTROL_INSET);

        layers.push(close).into()
    }
}

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
            mouse::Interaction::ResizingDiagonallyDown,
            Horizontal::Right,
            Vertical::Bottom,
        ),
        Direction::SouthWest => (
            RESIZE_CORNER.into(),
            RESIZE_CORNER.into(),
            mouse::Interaction::ResizingDiagonallyUp,
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
