//! Application-owned chrome for the undecorated app and gallery windows.

use iced::{
    Background, Border, Element, Fill, Subscription, event, keyboard, widget::container, window,
};

use iced::{
    alignment::{Horizontal, Vertical},
    mouse,
    widget::{Space, mouse_area, stack},
    window::Direction,
};

use next_ui::{
    Icon, theme,
    widget::{Button, ButtonKind, HeaderBar},
};

const RESIZE_EDGE: f32 = 6.0;
const RESIZE_CORNER: f32 = 12.0;
const PANEL_INSET: [f32; 2] = [6.0, 8.0];
const WINDOW_CONTROL_INSET: [f32; 2] = [22.0, 20.0];
const WINDOW_CONTROL_SIZE: f32 = 32.0;

pub(crate) const WINDOW_CONTROL_AT_START: bool = cfg!(target_os = "macos");

/// A semantic window-frame interaction for the application to handle.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Action {
    /// Begin dragging the current window.
    Drag,
    /// Begin resizing the current window from an edge or corner.
    Resize(window::Direction),
    /// Ask the application to run its shutdown lifecycle.
    RequestClose,
    /// Move to the previous global focus target.
    FocusPrevious,
    /// Move to the next global focus target.
    FocusNext,
}

/// Listens for application-window requests and unhandled keyboard traversal.
pub(crate) fn subscription() -> Subscription<Action> {
    iced::event::listen_with(|event, status, _| subscription_action(event, status))
}

fn subscription_action(event: iced::Event, status: event::Status) -> Option<Action> {
    match event {
        iced::Event::Window(window::Event::CloseRequested) => Some(Action::RequestClose),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Tab),
            modifiers,
            repeat: false,
            ..
        }) if status == event::Status::Ignored => Some(if modifiers.shift() {
            Action::FocusPrevious
        } else {
            Action::FocusNext
        }),
        _ => None,
    }
}

/// An undecorated application frame with resize edges and a close control.
pub(crate) fn window_frame<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_action: impl Fn(Action) -> Message,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = container(content)
        .width(Fill)
        .height(Fill)
        .padding(PANEL_INSET)
        .style(|current_theme| {
            let colors = theme::colors(current_theme);

            container::Style {
                background: Some(Background::Color(colors.window)),
                border: Border::default()
                    .rounded(6)
                    .color(colors.window_border)
                    .width(1),
                ..container::Style::default()
            }
        })
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
        layers = layers.push(resize_edge(direction, on_action(Action::Resize(direction))))
    }

    let close = Button::icon_only("Close window", Icon::Cross)
        .diameter(WINDOW_CONTROL_SIZE)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent)
        .on_press(on_action(Action::RequestClose));
    let close = container(close)
        .width(Fill)
        .height(Fill)
        .align_x(if WINDOW_CONTROL_AT_START {
            Horizontal::Left
        } else {
            Horizontal::Right
        })
        .align_y(Vertical::Top)
        .padding(WINDOW_CONTROL_INSET);

    layers.push(close).into()
}

pub(crate) fn header<'a, Message: Clone + 'a>(
    on_drag: Message,
    owns_window_control: bool,
    build: impl FnOnce(HeaderBar<'a, Message>) -> HeaderBar<'a, Message>,
) -> Element<'a, Message> {
    let header = HeaderBar::new(on_drag);

    let header = if !owns_window_control {
        build(header)
    } else {
        let spacer = Space::new()
            .width(WINDOW_CONTROL_SIZE)
            .height(WINDOW_CONTROL_SIZE);

        if WINDOW_CONTROL_AT_START {
            build(header.start(spacer))
        } else {
            build(header).end(spacer)
        }
    };

    header.into()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(modifiers: keyboard::Modifiers, repeat: bool) -> iced::Event {
        let key = keyboard::Key::Named(keyboard::key::Named::Tab);

        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Tab),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            key,
            repeat,
        })
    }

    #[test]
    fn only_ignored_non_repeated_tabs_request_global_focus_traversal() {
        assert!(matches!(
            subscription_action(
                tab(keyboard::Modifiers::NONE, false),
                event::Status::Ignored
            ),
            Some(Action::FocusNext)
        ));
        assert!(matches!(
            subscription_action(
                tab(keyboard::Modifiers::SHIFT, false),
                event::Status::Ignored
            ),
            Some(Action::FocusPrevious)
        ));
        assert!(
            subscription_action(
                tab(keyboard::Modifiers::NONE, false),
                event::Status::Captured
            )
            .is_none()
        );
        assert!(
            subscription_action(tab(keyboard::Modifiers::NONE, true), event::Status::Ignored)
                .is_none()
        );
    }

    #[test]
    fn close_requests_are_reported_even_when_a_modal_captures_them() {
        assert!(matches!(
            subscription_action(
                iced::Event::Window(window::Event::CloseRequested),
                event::Status::Captured,
            ),
            Some(Action::RequestClose)
        ));
    }
}
