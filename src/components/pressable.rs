use iced::{
    Background, Color, Element, Event, Length, Padding, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
    widget::button,
    window,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Active,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

pub(crate) struct Pressable<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    width: Length,
    height: Length,
    padding: Padding,
    style: Box<dyn Fn(&Theme, Status) -> button::Style + 'a>,
    last_status: Option<Status>,
}

impl<'a, Message> Pressable<'a, Message> {
    pub(crate) fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
            style: Box::new(|_, _| button::Style::default()),
            last_status: None,
        }
    }

    pub(crate) fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub(crate) fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub(crate) fn style(mut self, style: impl Fn(&Theme, Status) -> button::Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }
}

#[derive(Debug, Default)]
struct State {
    pressed: bool,
    focused: bool,
}

impl iced::advanced::widget::operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.pressed = false;
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Pressable<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();

        if self.on_press.is_some() {
            operation.focusable(None, layout.bounds(), state);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("pressable content layout"),
                renderer,
                operation,
            );
        });
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
            layout.children().next().expect("pressable content layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let child_captured = shell.is_event_captured();
        let enabled = self.on_press.is_some();
        let hovered = enabled && cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<State>();

        if !child_captured {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) => {
                    state.focused = hovered;

                    if hovered {
                        state.pressed = true;
                        shell.capture_event();
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. }) => {
                    if state.pressed {
                        state.pressed = false;

                        if hovered && let Some(message) = &self.on_press {
                            shell.publish(message.clone());
                        }

                        shell.capture_event();
                    }
                }
                Event::Touch(touch::Event::FingerLost { .. }) => state.pressed = false,
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Enter | key::Named::Space),
                    repeat: false,
                    ..
                }) if enabled && state.focused => {
                    state.pressed = true;
                    shell.capture_event();
                }
                Event::Keyboard(keyboard::Event::KeyReleased {
                    key: keyboard::Key::Named(key::Named::Enter | key::Named::Space),
                    ..
                }) if enabled && state.focused && state.pressed => {
                    state.pressed = false;

                    if let Some(message) = &self.on_press {
                        shell.publish(message.clone());
                    }

                    shell.capture_event();
                }
                _ => {}
            }
        }

        let status = status(enabled, hovered, state);

        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            self.last_status = Some(status);
        } else if self.last_status.is_some_and(|last| last != status) {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let status = self.last_status.unwrap_or_else(|| {
            status(
                self.on_press.is_some(),
                cursor.is_over(layout.bounds()),
                state,
            )
        });
        let style = (self.style)(theme, status);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: style.border,
                    shadow: style.shadow,
                    snap: style.snap,
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color,
            },
            layout.children().next().expect("pressable content layout"),
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let child = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("pressable content layout"),
            cursor,
            viewport,
            renderer,
        );

        if child != mouse::Interaction::default() {
            child
        } else if self.on_press.is_some() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("pressable content layout"),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Pressable<'a, Message>> for Element<'a, Message> {
    fn from(pressable: Pressable<'a, Message>) -> Self {
        Element::new(pressable)
    }
}

fn status(enabled: bool, hovered: bool, state: &State) -> Status {
    if !enabled {
        Status::Disabled
    } else if state.pressed {
        Status::Pressed
    } else if hovered {
        Status::Hovered
    } else if state.focused {
        Status::Focused
    } else {
        Status::Active
    }
}

#[cfg(test)]
mod tests {
    use super::{State, Status, status};

    #[test]
    fn interaction_status_has_stable_priority() {
        let active = State::default();
        assert_eq!(status(false, true, &active), Status::Disabled);
        assert_eq!(status(true, true, &active), Status::Hovered);

        let focused = State {
            focused: true,
            ..State::default()
        };
        assert_eq!(status(true, false, &focused), Status::Focused);

        let pressed = State {
            pressed: true,
            focused: true,
        };
        assert_eq!(status(true, true, &pressed), Status::Pressed);
    }
}
