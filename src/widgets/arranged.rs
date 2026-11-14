//! Containers that only decide where their children go.
//!
//! An iced widget that lays out children must also pass every event, draw,
//! operation and overlay on to them. [`Arranged`] does that passing once, so
//! a layout is only an [`Arrangement`]: how big the container is, and where
//! each child sits within it.

use iced::{
    Event, Length, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
};

use crate::{Element, Theme};

/// How a container places its children.
pub(crate) trait Arrangement {
    /// The container's size, as any widget reports it.
    fn size(&self) -> Size<Length>;

    /// Lays out `children`, each with its tree in `trees`, within `limits`,
    /// returning the container's node with one child node per child.
    fn layout<Message>(
        &self,
        children: &mut [Element<'_, Message>],
        trees: &mut [Tree],
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node;

    /// Places `children` with this arrangement.
    fn arrange<'a, Message: 'a>(
        self,
        children: impl IntoIterator<Item = Element<'a, Message>>,
    ) -> Element<'a, Message>
    where
        Self: Sized + 'a,
    {
        Element::new(Arranged {
            children: children.into_iter().collect(),
            arrangement: self,
        })
    }
}

/// Children placed by an [`Arrangement`], passed everything else unchanged.
struct Arranged<'a, Message, A> {
    children: Vec<Element<'a, Message>>,
    arrangement: A,
}

impl<Message, A: Arrangement> Widget<Message, Theme, iced::Renderer> for Arranged<'_, Message, A> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        self.arrangement.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.arrangement
            .layout(&mut self.children, &mut tree.children, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
