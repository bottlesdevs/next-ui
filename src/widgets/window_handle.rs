//! Regions of the window that move it when pressed, like GTK's
//! `GtkWindowHandle`.
//!
//! A [`WindowHandle`] only marks its bounds. The window frame owns the move:
//! when a press reaches it unhandled, it asks the tree with [`is_over`]
//! whether the press landed on a handle.

use std::any::Any;

use iced::{
    Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Id, Operation, Tree, tree},
    },
};

/// Marks `content` as a handle that moves the window.
pub struct WindowHandle<'a, Message> {
    content: Element<'a, Message>,
}

impl<'a, Message> WindowHandle<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// The state a [`WindowHandle`] reports through [`Operation::custom`], which
/// any widget may use. [`is_over`] counts a report as a handle only when its
/// state is a `Marker`. It is private, so nothing else can report as a
/// handle.
struct Marker;

/// Whether `point` lands on a handle in `content`.
pub(crate) fn is_over<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    point: Point,
) -> bool {
    struct Query {
        point: Point,
        hit: bool,
    }

    impl Operation for Query {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
            self.hit |= state.is::<Marker>() && bounds.contains(self.point);
        }
    }

    let mut query = Query { point, hit: false };
    content
        .as_widget_mut()
        .operate(tree, layout, renderer, &mut query);
    query.hit
}

/// An [`Operation`] filter: wraps another operation and passes on only what
/// handles report, through [`Operation::custom`]. Focus, scrolling, text
/// input and every other operation are dropped. It wraps itself around each
/// level it traverses, so the filter covers the whole subtree.
///
/// [`inert`](super::inert::inert) runs its content's operations through it,
/// so blocked content takes no focus, yet its handles still move the window.
pub(crate) struct HandlesOnly<'a>(pub &'a mut dyn Operation);

impl Operation for HandlesOnly<'_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        self.0
            .traverse(&mut |operation| operate(&mut HandlesOnly(operation)));
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
        self.0.custom(id, bounds, state);
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for WindowHandle<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
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
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.custom(None, layout.bounds(), &mut Marker);
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
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
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<WindowHandle<'a, Message>> for Element<'a, Message> {
    fn from(handle: WindowHandle<'a, Message>) -> Self {
        Element::new(handle)
    }
}
