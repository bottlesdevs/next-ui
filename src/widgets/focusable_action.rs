use iced::{
    Color, Element, Event, Length, Padding, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Id, Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::button,
    window,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ActionState {
    pub(crate) enabled: bool,
    pub(crate) hovered: bool,
    pub(crate) pressed: bool,
    pub(crate) focused: bool,
    pub(crate) selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressOrigin {
    Mouse,
    Touch(touch::Finger),
    Key(key::Named),
}

#[derive(Debug, Default)]
struct Interaction {
    press: Option<PressOrigin>,
    focused: bool,
    hovered: bool,
}

impl Interaction {
    fn sync_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.press = None;
            self.focused = false;
            self.hovered = false;
        }
    }

    fn update<Message>(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        enabled: bool,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let before = (self.press, self.focused, self.hovered);
        let pointer = event_cursor(event, cursor);
        let hovered = enabled && pointer.is_over(bounds);
        let mut activated = false;

        if !enabled || matches!(event, Event::Window(window::Event::Unfocused)) {
            self.press = None;
            self.focused = false;
            self.hovered = false;
        } else {
            if matches!(event, Event::Mouse(mouse::Event::CursorLeft)) {
                self.hovered = false;
            } else if pointer != mouse::Cursor::Unavailable {
                self.hovered = hovered;
            }

            if shell.is_event_captured() {
                if self
                    .press
                    .is_some_and(|press| event_origin(event) == Some(press))
                {
                    self.press = None;
                }
            } else if let Event::Touch(touch::Event::FingerLost { id, .. }) = event
                && self.press == Some(PressOrigin::Touch(*id))
            {
                self.press = None;
                shell.capture_event();
            } else if let Some(press) = press_started(event) {
                let can_start = match press {
                    PressOrigin::Key(_) => self.focused,
                    PressOrigin::Mouse | PressOrigin::Touch(_) => hovered,
                };

                if can_start && self.press.is_none() {
                    self.press = Some(press);
                    shell.capture_event();
                }
            } else if let Some(release) = press_released(event)
                && self.press == Some(release)
            {
                self.press = None;
                activated = match release {
                    PressOrigin::Key(_) => self.focused,
                    PressOrigin::Mouse | PressOrigin::Touch(_) => hovered,
                };
                shell.capture_event();
            }

            if matches!(
                event,
                Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
            ) {
                self.hovered = false;
            }
        }

        if !matches!(event, Event::Window(window::Event::RedrawRequested(_)))
            && before != (self.press, self.focused, self.hovered)
        {
            shell.request_redraw();
        }

        activated
    }

    fn state(
        &self,
        enabled: bool,
        selected: bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> ActionState {
        ActionState {
            enabled,
            hovered: enabled
                && if cursor == mouse::Cursor::Unavailable {
                    self.hovered
                } else {
                    cursor.is_over(bounds)
                },
            pressed: enabled && self.press.is_some(),
            focused: enabled && self.focused,
            selected,
        }
    }
}

impl operation::Focusable for Interaction {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.press = None;
    }
}

type StyleFn<'a> = dyn Fn(&Theme, ActionState) -> button::Style + 'a;

pub(crate) struct FocusableAction<'a, Message> {
    content: Element<'a, Message>,
    id: Option<Id>,
    on_press: Option<Message>,
    selected: bool,
    width: Length,
    height: Length,
    padding: Padding,
    style: Box<StyleFn<'a>>,
}

impl<'a, Message> FocusableAction<'a, Message> {
    pub(crate) fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            id: None,
            on_press: None,
            selected: false,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
            style: Box::new(|theme, _| button::Style {
                text_color: crate::theme::colors(theme).text,
                ..button::Style::default()
            }),
        }
    }

    pub(crate) fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub(crate) fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
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

    pub(crate) fn style(
        mut self,
        style: impl Fn(&Theme, ActionState) -> button::Style + 'a,
    ) -> Self {
        self.style = Box::new(style);
        self
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for FocusableAction<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Interaction>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Interaction::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        tree.state
            .downcast_mut::<Interaction>()
            .sync_enabled(self.on_press.is_some());
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
        if self.on_press.is_some() {
            operation.focusable(
                self.id.as_ref(),
                layout.bounds(),
                tree.state.downcast_mut::<Interaction>(),
            );
        }

        operation.container(self.id.as_ref(), layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("action content layout"),
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
        let content_layout = layout.children().next().expect("action content layout");
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let visible_bounds = layout.bounds().intersection(viewport).unwrap_or(Rectangle {
            x: layout.bounds().x,
            y: layout.bounds().y,
            width: 0.0,
            height: 0.0,
        });

        if tree.state.downcast_mut::<Interaction>().update(
            event,
            visible_bounds,
            cursor,
            self.on_press.is_some(),
            shell,
        ) && let Some(message) = &self.on_press
        {
            shell.publish(message.clone());
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<Interaction>().state(
            self.on_press.is_some(),
            self.selected,
            layout.bounds(),
            cursor,
        );
        let style = (self.style)(theme, state);

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
                    .unwrap_or(iced::Background::Color(Color::TRANSPARENT)),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color,
            },
            layout.children().next().expect("action content layout"),
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
            layout.children().next().expect("action content layout"),
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
            layout.children().next().expect("action content layout"),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<FocusableAction<'a, Message>> for Element<'a, Message> {
    fn from(action: FocusableAction<'a, Message>) -> Self {
        Element::new(action)
    }
}

fn press_started(event: &Event) -> Option<PressOrigin> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => Some(PressOrigin::Mouse),
        Event::Touch(touch::Event::FingerPressed { id, .. }) => Some(PressOrigin::Touch(*id)),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named @ (key::Named::Enter | key::Named::Space)),
            repeat: false,
            ..
        }) => Some(PressOrigin::Key(*named)),
        _ => None,
    }
}

fn press_released(event: &Event) -> Option<PressOrigin> {
    match event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(PressOrigin::Mouse),
        Event::Touch(touch::Event::FingerLifted { id, .. }) => Some(PressOrigin::Touch(*id)),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: keyboard::Key::Named(named @ (key::Named::Enter | key::Named::Space)),
            ..
        }) => Some(PressOrigin::Key(*named)),
        _ => None,
    }
}

fn event_origin(event: &Event) -> Option<PressOrigin> {
    match event {
        Event::Mouse(
            mouse::Event::ButtonPressed(mouse::Button::Left)
            | mouse::Event::ButtonReleased(mouse::Button::Left),
        ) => Some(PressOrigin::Mouse),
        Event::Touch(
            touch::Event::FingerPressed { id, .. }
            | touch::Event::FingerMoved { id, .. }
            | touch::Event::FingerLifted { id, .. }
            | touch::Event::FingerLost { id, .. },
        ) => Some(PressOrigin::Touch(*id)),
        Event::Keyboard(
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(named @ (key::Named::Enter | key::Named::Space)),
                ..
            }
            | keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(named @ (key::Named::Enter | key::Named::Space)),
                ..
            },
        ) => Some(PressOrigin::Key(*named)),
        _ => None,
    }
}

fn event_cursor(event: &Event, cursor: mouse::Cursor) -> mouse::Cursor {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => mouse::Cursor::Available(*position),
        _ => cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_primary_inputs_start_and_finish_activation() {
        let enter = key::Named::Enter;
        let escape = key::Named::Escape;
        let enter_pressed = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(enter),
            modified_key: keyboard::Key::Named(enter),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Enter),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });
        let escape_pressed = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(escape),
            modified_key: keyboard::Key::Named(escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });

        assert_eq!(press_started(&enter_pressed), Some(PressOrigin::Key(enter)));
        assert_eq!(press_started(&escape_pressed), None);
        assert_eq!(
            press_started(&Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Right,
            ))),
            None
        );
    }
}
