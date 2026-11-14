//! Chrome for undecorated application windows.

use iced::{
    Background, Border, Element, Fill, Subscription, event, keyboard, widget::container, window,
};

use iced::{
    Event, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, overlay, renderer,
        widget::{Operation, Tree},
    },
    alignment::{Horizontal, Vertical},
    mouse,
    widget::{Space, mouse_area, stack},
    window::Direction,
};

use crate::{
    Icon, theme,
    widget::{HeaderBar, header, header_button},
    widgets::window_handle,
};

const RESIZE_EDGE: f32 = 6.0;
const RESIZE_CORNER: f32 = 12.0;
const PANEL_INSET: [f32; 2] = [6.0, 8.0];

pub const WINDOW_CONTROL_AT_START: bool = cfg!(target_os = "macos");

/// A semantic window-frame interaction for the application to handle.
#[derive(Debug, Clone, Copy)]
pub enum Action {
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
pub fn subscription() -> Subscription<Action> {
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
pub fn window_frame<'a, Message: Clone + 'a>(
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
                background: Some(Background::Color(colors.chrome)),
                border: Border::default()
                    .rounded(crate::widgets::radius::ROW)
                    .color(colors.chrome_border)
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

    let close = container(
        header_button("Close window", Icon::Cross).on_press(on_action(Action::RequestClose)),
    )
    .height(header::HEIGHT)
    .align_y(Vertical::Center);
    let padding = iced::Padding::from(PANEL_INSET);
    let padding = if WINDOW_CONTROL_AT_START {
        padding.left(PANEL_INSET[1] + header::INSET)
    } else {
        padding.right(PANEL_INSET[1] + header::INSET)
    };
    let close = container(close)
        .width(Fill)
        .height(Fill)
        .align_x(if WINDOW_CONTROL_AT_START {
            Horizontal::Left
        } else {
            Horizontal::Right
        })
        .align_y(Vertical::Top)
        .padding(padding);

    Element::new(Frame {
        content: layers.push(close).into(),
        on_drag: on_action(Action::Drag),
    })
}

/// The root of the window: wraps everything [`window_frame`] builds, the
/// app's content, the resize edges and the close control.
///
/// It owns moving the window. When a left press reaches it unhandled and
/// lands within the bounds of a
/// [`WindowHandle`](crate::widgets::window_handle::WindowHandle), including
/// one in inert content, it publishes `on_drag` for the app to start the
/// move.
struct Frame<'a, Message> {
    content: Element<'a, Message>,
    on_drag: Message,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Frame<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if !shell.is_event_captured()
            && matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            )
            && let Some(position) = cursor.position()
            && window_handle::is_over(
                &mut self.content,
                &mut tree.children[0],
                layout,
                renderer,
                position,
            )
        {
            shell.publish(self.on_drag.clone());
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

pub fn header<'a, Message: Clone + 'a>(
    owns_window_control: bool,
    build: impl FnOnce(HeaderBar<'a, Message>) -> HeaderBar<'a, Message>,
) -> Element<'a, Message> {
    let header = HeaderBar::default();

    let header = if !owns_window_control {
        build(header)
    } else {
        let spacer = Space::new().width(header::CONTROL).height(header::CONTROL);

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
