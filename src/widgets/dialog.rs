use iced::{
    Background, Element, Event, Fill, Length, Point, Rectangle, Size, Theme,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    event, keyboard, touch,
    widget::{container, stack},
    window,
};

use crate::theme;

use super::{Control, control::focus_first_descendant, event_cursor};

const DIALOG_MAX_WIDTH: f32 = 360.0;

/// Arbitrary content presented by a [`WindowModal`].
pub struct Dialog<'a, Message> {
    content: Element<'a, Message>,
    on_dismiss: Message,
}

impl<'a, Message: 'a> Dialog<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>, on_dismiss: Message) -> Self {
        Self {
            content: container(content)
                .max_width(DIALOG_MAX_WIDTH)
                .padding(24)
                .style(theme::panel)
                .into(),
            on_dismiss,
        }
    }

    pub fn map<Other>(self, mapper: impl Fn(Message) -> Other + 'a) -> Dialog<'a, Other>
    where
        Message: 'a,
        Other: 'a,
    {
        let on_dismiss = mapper(self.on_dismiss);

        Dialog {
            content: self.content.map(mapper),
            on_dismiss,
        }
    }
}

/// Presents one controlled dialog over a complete application window.
pub struct WindowModal<'a, Message> {
    base: Element<'a, Message>,
    dialog: Option<Dialog<'a, Message>>,
}

impl<'a, Message> WindowModal<'a, Message> {
    pub fn new(base: impl Into<Element<'a, Message>>) -> Self {
        Self {
            base: base.into(),
            dialog: None,
        }
    }

    pub fn dialog(mut self, dialog: Option<Dialog<'a, Message>>) -> Self {
        self.dialog = dialog;
        self
    }
}

impl<'a, Message: Clone + 'a> From<WindowModal<'a, Message>> for Element<'a, Message> {
    fn from(modal: WindowModal<'a, Message>) -> Self {
        let open = modal.dialog.is_some();
        let base = Control::new(modal.base)
            .width(Fill)
            .height(Fill)
            .sensitive(!open);
        let mut layers = stack![base].width(Fill).height(Fill);

        if let Some(dialog) = modal.dialog {
            layers = layers.push(Element::new(ModalLayer {
                content: dialog.content,
                on_dismiss: dialog.on_dismiss,
            }));
        }

        layers.into()
    }
}

struct ModalLayer<'a, Message> {
    content: Element<'a, Message>,
    on_dismiss: Message,
}

struct State {
    focus_pending: bool,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for ModalLayer<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            focus_pending: true,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let content = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, size).loose(),
        );
        let content_size = content.size();
        let content = content.move_to(Point::new(
            (size.width - content_size.width) / 2.0,
            (size.height - content_size.height) / 2.0,
        ));
        let node = layout::Node::with_children(size, vec![content]);

        if tree.state.downcast_ref::<State>().focus_pending {
            let content_layout = Layout::new(&node)
                .children()
                .next()
                .expect("dialog content layout");
            focus_first_descendant(
                &mut self.content,
                &mut tree.children[0],
                content_layout,
                renderer,
            );
            tree.state.downcast_mut::<State>().focus_pending = false;
        }

        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().expect("dialog content layout"),
            renderer,
            operation,
        );
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
        let content_layout = layout.children().next().expect("dialog content layout");
        let cursor = event_cursor(event, cursor);

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

        if requests_dismissal(event, shell.event_status(), content_layout.bounds(), cursor) {
            shell.publish(self.on_dismiss.clone());
            shell.capture_event();
            return;
        }

        if shell.event_status() == event::Status::Captured {
            return;
        }

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                // Repeated Escape presses are still captured while the dismissal message is routed.
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            }) => {
                cycle_focus(
                    &mut self.content,
                    &mut tree.children[0],
                    content_layout,
                    renderer,
                    modifiers.shift(),
                );
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(iced::mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. }) => shell.capture_event(),
            Event::Window(window::Event::CloseRequested) => shell.capture_event(),
            Event::Keyboard(_) | Event::Mouse(_) | Event::Touch(_) | Event::InputMethod(_) => {
                shell.capture_event();
            }
            Event::Window(_) => {}
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
        let interaction = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("dialog content layout"),
            cursor,
            viewport,
            renderer,
        );

        if interaction == mouse::Interaction::None {
            mouse::Interaction::Idle
        } else {
            interaction
        }
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
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            Background::Color(theme::SCRIM),
        );
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().expect("dialog content layout"),
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
        translation: iced::Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("dialog content layout"),
            renderer,
            viewport,
            translation,
        )
    }
}

fn requests_dismissal(
    event: &Event,
    child_status: event::Status,
    content: Rectangle,
    cursor: mouse::Cursor,
) -> bool {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            repeat: false,
            ..
        }) => true,
        Event::Mouse(iced::mouse::Event::ButtonPressed(_))
            if child_status == event::Status::Ignored =>
        {
            !cursor.is_over(content)
        }
        Event::Touch(touch::Event::FingerPressed { position, .. })
            if child_status == event::Status::Ignored =>
        {
            !content.contains(*position)
        }
        _ => false,
    }
}

fn cycle_focus<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    previous: bool,
) {
    let mut count = operation::focusable::count();
    content.as_widget_mut().operate(
        tree,
        layout,
        renderer,
        &mut operation::black_box(&mut count),
    );
    let operation::Outcome::Some(count) = Operation::finish(&count) else {
        return;
    };
    if count.total == 0 {
        return;
    }

    let target = if previous {
        count.focused.map_or(count.total - 1, |focused| {
            (focused + count.total - 1) % count.total
        })
    } else {
        count
            .focused
            .map_or(0, |focused| (focused + 1) % count.total)
    };
    content.as_widget_mut().operate(
        tree,
        layout,
        renderer,
        &mut FocusIndex { current: 0, target },
    );
}

struct FocusIndex {
    current: usize,
    target: usize,
}

impl Operation for FocusIndex {
    fn focusable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if self.current == self.target {
            state.focus();
        } else {
            state.unfocus();
        }
        self.current += 1;
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
}

#[cfg(test)]
mod tests {
    use iced::{advanced::widget::Tree, widget::Space};

    use super::*;
    use crate::widgets::Interaction;

    #[derive(Clone)]
    enum Message {
        Dismiss,
    }

    fn host(open: bool) -> Element<'static, Message> {
        WindowModal::new(Control::new(Space::new()).on_press(Message::Dismiss))
            .dialog(open.then(|| Dialog::new(Space::new(), Message::Dismiss)))
            .into()
    }

    fn base_interaction(tree: &mut Tree) -> &mut Interaction {
        tree.children[0].children[0]
            .state
            .downcast_mut::<Interaction>()
    }

    #[test]
    fn opening_and_closing_retains_the_base_tree() {
        use iced::advanced::widget::operation::Focusable;

        let closed = host(false);
        let mut tree = Tree::new(&closed);
        base_interaction(&mut tree).focus();

        tree.diff(&host(true));
        assert_eq!(tree.children.len(), 2);
        assert!(base_interaction(&mut tree).is_focused());

        tree.diff(&host(false));
        assert_eq!(tree.children.len(), 1);
        assert!(base_interaction(&mut tree).is_focused());
    }

    #[test]
    fn dismissal_is_limited_to_escape_and_outside_presses() {
        let content = Rectangle::new(Point::new(20.0, 20.0), Size::new(100.0, 100.0));
        let inside = mouse::Cursor::Available(Point::new(40.0, 40.0));
        let outside = mouse::Cursor::Available(Point::new(10.0, 10.0));

        assert!(!requests_dismissal(
            &Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left,)),
            event::Status::Ignored,
            content,
            inside,
        ));
        assert!(requests_dismissal(
            &Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left,)),
            event::Status::Ignored,
            content,
            outside,
        ));

        let touch = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(0),
            position: Point::new(10.0, 10.0),
        });
        assert!(requests_dismissal(
            &touch,
            event::Status::Ignored,
            content,
            inside,
        ));
        assert!(requests_dismissal(
            &key_pressed(keyboard::key::Named::Escape, false),
            event::Status::Captured,
            content,
            inside,
        ));
        assert!(!requests_dismissal(
            &key_pressed(keyboard::key::Named::Escape, true),
            event::Status::Ignored,
            content,
            inside,
        ));
        assert!(!requests_dismissal(
            &Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left,)),
            event::Status::Captured,
            content,
            outside,
        ));
    }

    fn key_pressed(named: keyboard::key::Named, repeat: bool) -> Event {
        let key = keyboard::Key::Named(named);

        Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            key,
            repeat,
        })
    }
}
