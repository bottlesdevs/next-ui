use iced::{
    Background, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::button,
    window,
};
use std::{cell::Cell, rc::Rc};

/// A one-shot activation event from a [`Control`] to its composite owner.
/// It does not own persistent widget state.
pub(crate) type ActivationSignal = Rc<Cell<bool>>;

/// The independent states used to resolve a control's appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct State {
    pub(crate) sensitive: bool,
    pub(crate) actionable: bool,
    pub(crate) hovered: bool,
    pub(crate) pressed: bool,
    pub(crate) focused: bool,
    pub(crate) focus_within: bool,
    pub(crate) selected: bool,
    pub(crate) expanded: bool,
    pub(crate) keyboard_highlighted: bool,
}

/// The complete appearance of a [`Control`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Style {
    pub(crate) background: Option<Background>,
    pub(crate) text_color: Color,
    pub(crate) border: iced::Border,
    pub(crate) shadow: Shadow,
    pub(crate) snap: bool,
    /// A foreground fill drawn after the content, such as a disabled scrim.
    pub(crate) foreground: Option<Background>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            text_color: Color::BLACK,
            border: iced::Border::default(),
            shadow: Shadow::default(),
            snap: true,
            foreground: None,
        }
    }
}

impl From<button::Style> for Style {
    fn from(style: button::Style) -> Self {
        Self {
            background: style.background,
            text_color: style.text_color,
            border: style.border,
            shadow: style.shadow,
            snap: style.snap,
            foreground: None,
        }
    }
}

/// The result of applying an input event to an [`Interaction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Ignored,
    Captured,
    Activated,
}

/// Transient input and focus state shared by controls and composite widgets.
#[derive(Debug, Default)]
pub(crate) struct Interaction {
    pressed: bool,
    focused: bool,
    hovered: bool,
    descendant_focused: bool,
}

impl Interaction {
    pub(crate) fn update<Message>(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        sensitive: bool,
        actionable: bool,
        child_captured: bool,
        shell: &mut Shell<'_, Message>,
    ) -> Outcome {
        let previous = (self.pressed, self.focused, self.hovered);
        let pointer = event_cursor(event, cursor);
        let hovered = sensitive && pointer.is_over(bounds);
        let mut outcome = Outcome::Ignored;

        if !sensitive {
            self.pressed = false;
            self.focused = false;
            self.hovered = false;
            self.descendant_focused = false;
        } else {
            if matches!(event, Event::Mouse(mouse::Event::CursorLeft)) {
                self.hovered = false;
            } else if pointer != mouse::Cursor::Unavailable {
                self.hovered = hovered;
            }

            if !actionable {
                self.pressed = false;
                self.focused = false;
            } else if child_captured {
                if matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        | Event::Touch(touch::Event::FingerPressed { .. })
                ) {
                    self.focused = false;
                    self.pressed = false;
                } else if matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                        | Event::Touch(
                            touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }
                        )
                ) {
                    self.pressed = false;
                }
            } else {
                outcome = match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. }) => {
                        self.focused = false;

                        if hovered {
                            self.pressed = true;
                            Outcome::Captured
                        } else {
                            Outcome::Ignored
                        }
                    }
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerLifted { .. })
                        if self.pressed =>
                    {
                        self.pressed = false;

                        if hovered {
                            Outcome::Activated
                        } else {
                            Outcome::Captured
                        }
                    }
                    Event::Touch(touch::Event::FingerLost { .. }) => {
                        self.pressed = false;
                        Outcome::Ignored
                    }
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(key::Named::Enter | key::Named::Space),
                        repeat: false,
                        ..
                    }) if self.focused => {
                        self.pressed = true;
                        Outcome::Captured
                    }
                    Event::Keyboard(keyboard::Event::KeyReleased {
                        key: keyboard::Key::Named(key::Named::Enter | key::Named::Space),
                        ..
                    }) if self.focused && self.pressed => {
                        self.pressed = false;
                        Outcome::Activated
                    }
                    _ => Outcome::Ignored,
                };
            }

            if matches!(
                event,
                Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
            ) {
                self.hovered = false;
            }
        }

        if outcome != Outcome::Ignored {
            shell.capture_event();
        }

        if !matches!(event, Event::Window(window::Event::RedrawRequested(_)))
            && previous != (self.pressed, self.focused, self.hovered)
        {
            shell.request_redraw();
        }

        outcome
    }

    pub(crate) fn state(
        &self,
        sensitive: bool,
        actionable: bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> State {
        let focused = sensitive && actionable && self.focused;
        let hovered = sensitive
            && if cursor == mouse::Cursor::Unavailable {
                self.hovered
            } else {
                cursor.is_over(bounds)
            };

        State {
            sensitive,
            actionable,
            hovered,
            pressed: sensitive && actionable && self.pressed,
            focused,
            focus_within: sensitive && (focused || self.descendant_focused),
            selected: false,
            expanded: false,
            keyboard_highlighted: false,
        }
    }

    pub(crate) fn set_descendant_focused(&mut self, focused: bool) {
        self.descendant_focused = focused;
    }

    pub(crate) fn mouse_interaction(
        &self,
        sensitive: bool,
        actionable: bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if sensitive && actionable && cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
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
        self.pressed = false;
    }
}

/// A single-child control with shared input, focus, and styling behavior.
pub(crate) struct Control<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    activation: Option<ActivationSignal>,
    sensitive: bool,
    selected: bool,
    focus_first_descendant: bool,
    width: Length,
    height: Length,
    padding: Padding,
    style: Box<dyn Fn(&Theme, State) -> Style + 'a>,
}

impl<'a, Message> Control<'a, Message> {
    pub(crate) fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            activation: None,
            sensitive: true,
            selected: false,
            focus_first_descendant: false,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
            style: Box::new(|theme, _| Style {
                text_color: theme.palette().text,
                ..Style::default()
            }),
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

    pub(crate) fn activation_signal(mut self, signal: ActivationSignal) -> Self {
        self.activation = Some(signal);
        self
    }

    pub(crate) fn sensitive(mut self, sensitive: bool) -> Self {
        self.sensitive = sensitive;
        self
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn focus_first_descendant(mut self) -> Self {
        self.focus_first_descendant = true;
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

    pub(crate) fn style<S>(mut self, style: impl Fn(&Theme, State) -> S + 'a) -> Self
    where
        S: Into<Style>,
    {
        self.style = Box::new(move |theme, state| style(theme, state).into());
        self
    }

    fn actionable(&self) -> bool {
        self.on_press.is_some() || self.activation.is_some()
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Control<'_, Message> {
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
        if !self.sensitive {
            return;
        }

        let interaction = tree.state.downcast_mut::<Interaction>();

        if self.actionable() && !self.focus_first_descendant {
            operation.focusable(None, layout.bounds(), interaction);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("control content layout"),
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
        let content_layout = layout.children().next().expect("control content layout");

        if self.sensitive {
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

            if self.focus_first_descendant
                && !shell.is_event_captured()
                && event_cursor(event, cursor).is_over(layout.bounds())
                && matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        | Event::Touch(touch::Event::FingerPressed { .. })
                )
            {
                focus_first_descendant(
                    self.content.as_widget_mut(),
                    &mut tree.children[0],
                    content_layout,
                    renderer,
                );
                shell.request_redraw();
            }
        }

        let child_captured = shell.is_event_captured();
        let interaction = tree.state.downcast_mut::<Interaction>();

        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            interaction.set_descendant_focused(
                self.sensitive
                    && descendant_is_focused(
                        &mut self.content,
                        &mut tree.children[0],
                        content_layout,
                        renderer,
                    ),
            );
        }

        if interaction.update(
            event,
            layout.bounds(),
            cursor,
            self.sensitive,
            self.actionable(),
            child_captured,
            shell,
        ) == Outcome::Activated
        {
            if let Some(activation) = &self.activation {
                activation.set(true);
            }

            if let Some(message) = &self.on_press {
                shell.publish(message.clone());
            }
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
        let bounds = layout.bounds();
        let interaction = tree.state.downcast_ref::<Interaction>();
        let mut state = interaction.state(self.sensitive, self.actionable(), bounds, cursor);
        state.selected = self.selected;

        let style = (self.style)(theme, state);

        if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
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
            layout.children().next().expect("control content layout"),
            cursor,
            viewport,
        );

        if let Some(foreground) = style.foreground {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: Shadow::default(),
                    snap: style.snap,
                },
                foreground,
            );
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
        if !self.sensitive {
            return mouse::Interaction::default();
        }

        let child = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("control content layout"),
            cursor,
            viewport,
            renderer,
        );

        if child != mouse::Interaction::default() {
            child
        } else {
            tree.state.downcast_ref::<Interaction>().mouse_interaction(
                self.sensitive,
                self.actionable(),
                layout.bounds(),
                cursor,
            )
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
        self.sensitive.then(|| {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                layout.children().next().expect("control content layout"),
                renderer,
                viewport,
                translation,
            )
        })?
    }
}

impl<'a, Message: Clone + 'a> From<Control<'a, Message>> for Element<'a, Message> {
    fn from(control: Control<'a, Message>) -> Self {
        Element::new(control)
    }
}

pub(crate) fn descendant_is_focused<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> bool {
    let mut count = operation::focusable::count();
    content.as_widget_mut().operate(
        tree,
        layout,
        renderer,
        &mut operation::black_box(&mut count),
    );

    matches!(
        Operation::finish(&count),
        operation::Outcome::Some(count) if count.focused.is_some()
    )
}

pub(crate) fn focus_first_descendant<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) {
    content.operate(tree, layout, renderer, &mut FocusFirst(false));
}

struct FocusFirst(bool);

impl Operation for FocusFirst {
    fn focusable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if !self.0 {
            state.focus();
            self.0 = true;
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
}

pub(crate) fn event_cursor(event: &Event, cursor: mouse::Cursor) -> mouse::Cursor {
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
