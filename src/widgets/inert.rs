//! Taking a subtree out of interaction.
//!
//! Three separate properties are involved: whether a widget gets pointer
//! input, whether focus traversal reaches it, and whether it is disabled and
//! greyed out. iced widgets have no common base to hang such properties on,
//! so here they are wrapper widgets around a subtree.
//!
//! [`inert`] blocks both pointer input and focus, because every user (the
//! content behind a dialog, the base behind a side panel, and a disabled
//! group) needs both at once. [`sensitive`](super::sensitive::sensitive) is
//! `inert` plus the disabled look, which comes from
//! [`Theme::disabled`](crate::Theme::disabled). Split `inert` into separate
//! wrappers when something needs one without the other, for example:
//!
//! - **No focus, still clickable**: an area the pointer can use but Tab
//!   should skip. Pass input on, but drop focus operations, as
//!   [`HandlesOnly`] does.
//! - **Click-through**: an overlay the pointer passes through to what is
//!   below, while the keyboard can still reach it. Drop pointer events and
//!   report no mouse interaction, but pass operations on.

use iced::{
    Length, Rectangle, Size,
    advanced::{
        Layout, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, tree},
    },
};

use super::window_handle::HandlesOnly;
use crate::{Element, Theme};

/// Shows `content` but takes it out of interaction: it gets no input, focus
/// or overlays while something above it is modal. Its window handles still
/// move the window.
///
/// The content's tree is kept as is, so wrapping and unwrapping it loses no
/// state.
pub(crate) fn inert<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(Inert {
        content: content.into(),
    })
}

struct Inert<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Inert<'_, Message> {
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
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut HandlesOnly(operation));
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            tree,
            renderer,
            theme,
            style,
            layout,
            mouse::Cursor::Unavailable,
            viewport,
        );
    }
}
