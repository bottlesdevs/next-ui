use iced::{
    Background, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Id, Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::button,
    window,
};
/// The independent states used to resolve a control's appearance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ActionState {
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

/// The complete appearance of an [`ActionSurface`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActionStyle {
    pub(crate) background: Option<Background>,
    pub(crate) text_color: Color,
    pub(crate) border: iced::Border,
    pub(crate) shadow: Shadow,
    pub(crate) snap: bool,
    /// A foreground fill drawn after the content, such as a disabled scrim.
    pub(crate) foreground: Option<Background>,
}

impl Default for ActionStyle {
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

impl From<button::Style> for ActionStyle {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressOrigin {
    Mouse(mouse::Button),
    Touch(touch::Finger),
    Key(key::Named),
}

/// Transient input and focus state shared by controls and composite widgets.
#[derive(Debug, Default)]
pub(crate) struct Interaction {
    press: Option<PressOrigin>,
    focused: bool,
    hovered: bool,
    descendant_focused: bool,
}

impl Interaction {
    fn sync_enabled(&mut self, sensitive: bool, actionable: bool) {
        if !sensitive {
            self.press = None;
            self.focused = false;
            self.hovered = false;
            self.descendant_focused = false;
        } else if !actionable {
            self.press = None;
            self.focused = false;
        }
    }

    pub(crate) fn update<Message>(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        sensitive: bool,
        actionable: bool,
        shell: &mut Shell<'_, Message>,
    ) -> Outcome {
        let child_captured = shell.is_event_captured();
        let previous = (self.press, self.focused, self.hovered);
        let pointer = event_cursor(event, cursor);
        let hovered = sensitive && pointer.is_over(bounds);
        let mut outcome = Outcome::Ignored;

        if !sensitive || matches!(event, Event::Window(window::Event::Unfocused)) {
            self.press = None;
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
                self.press = None;
                self.focused = false;
            } else if child_captured {
                if self
                    .press
                    .is_some_and(|press| event_origin(event) == Some(press))
                {
                    self.press = None;
                }

                if matches!(
                    press_started(event),
                    Some(PressOrigin::Mouse(_) | PressOrigin::Touch(_))
                ) {
                    self.focused = false;
                }
            } else if let Some(press) = press_started(event) {
                if matches!(press, PressOrigin::Mouse(_) | PressOrigin::Touch(_)) {
                    self.focused = false;

                    if matches!(self.press, Some(PressOrigin::Key(_))) {
                        self.press = None;
                    }
                }

                let can_start = match press {
                    PressOrigin::Key(_) => self.focused,
                    PressOrigin::Mouse(_) | PressOrigin::Touch(_) => hovered,
                };

                if can_start && self.press.is_none() {
                    self.press = Some(press);
                    outcome = Outcome::Captured;
                }
            } else if let Some(release) = press_released(event) {
                if self.press == Some(release) {
                    self.press = None;
                    let released_over_action = match release {
                        PressOrigin::Mouse(_) | PressOrigin::Touch(_) => hovered,
                        PressOrigin::Key(_) => self.focused,
                    };
                    outcome = if released_over_action {
                        Outcome::Activated
                    } else {
                        Outcome::Captured
                    };
                }
            } else if let Event::Touch(touch::Event::FingerLost { id, .. }) = event
                && self.press == Some(PressOrigin::Touch(*id))
            {
                self.press = None;
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
            && previous != (self.press, self.focused, self.hovered)
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
    ) -> ActionState {
        let focused = sensitive && actionable && self.focused;
        let hovered = sensitive
            && if cursor == mouse::Cursor::Unavailable {
                self.hovered
            } else {
                cursor.is_over(bounds)
            };

        ActionState {
            sensitive,
            actionable,
            hovered,
            pressed: sensitive && actionable && self.press.is_some(),
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
        self.press = None;
    }
}

/// A single-child control with shared input, focus, and styling behavior.
type StyleFn<'a> = dyn Fn(&Theme, ActionState) -> ActionStyle + 'a;

pub(crate) struct ActionSurface<'a, Message> {
    content: Element<'a, Message>,
    id: Option<Id>,
    on_press: Option<Message>,
    sensitive: bool,
    selected: bool,
    width: Length,
    height: Length,
    padding: Padding,
    style: Box<StyleFn<'a>>,
}

impl<'a, Message> ActionSurface<'a, Message> {
    pub(crate) fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            id: None,
            on_press: None,
            sensitive: true,
            selected: false,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
            style: Box::new(|theme, _| ActionStyle {
                text_color: crate::theme::colors(theme).text,
                ..ActionStyle::default()
            }),
        }
    }

    pub(crate) fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub(crate) fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub(crate) fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
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

    pub(crate) fn style<S>(mut self, style: impl Fn(&Theme, ActionState) -> S + 'a) -> Self
    where
        S: Into<ActionStyle>,
    {
        self.style = Box::new(move |theme, state| style(theme, state).into());
        self
    }

    fn actionable(&self) -> bool {
        self.on_press.is_some()
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for ActionSurface<'_, Message> {
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
            .sync_enabled(self.sensitive, self.actionable());
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

        if self.actionable() {
            operation.focusable(self.id.as_ref(), layout.bounds(), interaction);
        }

        operation.container(self.id.as_ref(), layout.bounds());
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
        let visible_bounds = layout.bounds().intersection(viewport).unwrap_or(Rectangle {
            x: layout.bounds().x,
            y: layout.bounds().y,
            width: 0.0,
            height: 0.0,
        });

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
        }

        let interaction = tree.state.downcast_mut::<Interaction>();

        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            interaction.set_descendant_focused(
                self.sensitive
                    && descendant_is_focused(
                        self.content.as_widget_mut(),
                        &mut tree.children[0],
                        content_layout,
                        renderer,
                    ),
            );
        }

        if interaction.update(
            event,
            visible_bounds,
            cursor,
            self.sensitive,
            self.actionable(),
            shell,
        ) == Outcome::Activated
            && let Some(message) = &self.on_press
        {
            shell.publish(message.clone());
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

impl<'a, Message: Clone + 'a> From<ActionSurface<'a, Message>> for Element<'a, Message> {
    fn from(surface: ActionSurface<'a, Message>) -> Self {
        Element::new(surface)
    }
}

fn press_started(event: &Event) -> Option<PressOrigin> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            Some(PressOrigin::Mouse(mouse::Button::Left))
        }
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
        Event::Mouse(mouse::Event::ButtonReleased(button)) => Some(PressOrigin::Mouse(*button)),
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
            mouse::Event::ButtonPressed(button) | mouse::Event::ButtonReleased(button),
        ) => Some(PressOrigin::Mouse(*button)),
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

pub(crate) fn descendant_is_focused<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> bool {
    matches!(
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::count(),
        ),
        operation::Outcome::Some(operation::focusable::Count {
            focused: Some(_),
            ..
        })
    )
}

pub(crate) fn focus_first_descendant<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) {
    unfocus_descendants(content, tree, layout, renderer);
    move_descendant_focus(content, tree, layout, renderer, false);
}

pub(crate) fn focus_descendant<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    id: Id,
) {
    run_operation(
        content,
        tree,
        layout,
        renderer,
        operation::focusable::focus::<()>(id),
    );
}

pub(crate) fn unfocus_descendants<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) {
    run_operation(
        content,
        tree,
        layout,
        renderer,
        operation::focusable::unfocus::<()>(),
    );
}

pub(crate) fn cycle_descendant_focus<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    previous: bool,
) {
    move_descendant_focus(content, tree, layout, renderer, previous);

    if !descendant_is_focused(content, tree, layout, renderer) {
        move_descendant_focus(content, tree, layout, renderer, previous);
    }
}

fn move_descendant_focus<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    previous: bool,
) {
    if previous {
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::focus_previous::<()>(),
        );
    } else {
        run_operation(
            content,
            tree,
            layout,
            renderer,
            operation::focusable::focus_next::<()>(),
        );
    }
}

fn run_operation<Message, Output: 'static>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    operation: impl Operation<Output> + 'static,
) -> operation::Outcome<Output> {
    let mut operation: Box<dyn Operation<Output>> = Box::new(operation);

    loop {
        content.operate(
            tree,
            layout,
            renderer,
            &mut operation::black_box(operation.as_mut()),
        );

        match operation.finish() {
            operation::Outcome::Chain(next) => operation = next,
            outcome => return outcome,
        }
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
