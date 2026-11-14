//! Natural sizes: how wide content wants to be when nothing is imposed.
//!
//! Layout works in two steps: measure each widget's natural size, then
//! allocate the final sizes. iced has only `layout()`, so [`natural_layout`]
//! measures with a compressed layout, under which `Fill` resolves to a
//! widget's intrinsic size. It is the toolkit's one definition of natural.
//!
//! iced's own column, when it shrinks, gives a child that fills its width
//! only the width of its widest *non-filling* sibling, so a list whose rows
//! all fill, such as a menu, collapses. [`NaturalColumn`] is the allocation
//! step: every child at the widest natural width.
//!
//! Measuring is a real layout, so a widget that keeps state from layout must
//! take it from the last pass, which is the allocation.

use iced::{
    Event, Length, Point, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
};

use crate::{Element, Theme};

/// Lays `element` out at its natural width within `max`: compressed, so
/// whatever fills reports its intrinsic width, and wrapping text its
/// unwrapped line up to `max`.
pub(crate) fn natural_layout<Message>(
    element: &mut Element<'_, Message>,
    tree: &mut Tree,
    renderer: &iced::Renderer,
    max: Size,
) -> layout::Node {
    element.as_widget_mut().layout(
        tree,
        renderer,
        &layout::Limits::with_compression(Size::ZERO, max, Size::new(true, false)),
    )
}

/// Stacks `children` at the widest of their natural widths, between
/// `min_width` and `max_width`.
pub(crate) struct NaturalColumn<'a, Message> {
    children: Vec<Element<'a, Message>>,
    min_width: f32,
    max_width: f32,
}

impl<'a, Message> NaturalColumn<'a, Message> {
    pub(crate) fn new(children: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        Self {
            children: children.into_iter().collect(),
            min_width: 0.0,
            max_width: f32::INFINITY,
        }
    }

    pub(crate) fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    pub(crate) fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for NaturalColumn<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // A minimum beats the maximum; the space there is beats both.
        let max = Size::new(
            self.max_width.max(self.min_width).min(limits.max().width),
            limits.max().height,
        );

        let natural = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, tree)| natural_layout(child, tree, renderer, max).size().width)
            .fold(0.0, f32::max);
        let width = natural
            .max(self.min_width)
            .max(limits.min().width)
            .min(max.width);

        // Allocate: every child at that width.
        let allocate = layout::Limits::new(Size::new(width, 0.0), Size::new(width, max.height));
        let mut y = 0.0;
        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, tree)| {
                let node = child
                    .as_widget_mut()
                    .layout(tree, renderer, &allocate)
                    .move_to(Point::new(0.0, y));
                y += node.size().height;
                node
            })
            .collect();

        layout::Node::with_children(Size::new(width, y), nodes)
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

impl<'a, Message: 'a> From<NaturalColumn<'a, Message>> for Element<'a, Message> {
    fn from(column: NaturalColumn<'a, Message>) -> Self {
        Element::new(column)
    }
}
