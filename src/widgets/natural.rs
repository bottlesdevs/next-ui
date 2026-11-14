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
    Length, Point, Size,
    advanced::{layout, widget::Tree},
};

use super::arranged::Arrangement;
use crate::Element;

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

/// Stacks children at the widest of their natural widths, between
/// `min_width` and `max_width`, like a vertical box.
pub(crate) struct NaturalColumn {
    min_width: f32,
    max_width: f32,
}

impl NaturalColumn {
    pub(crate) fn new() -> Self {
        Self {
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

impl Arrangement for NaturalColumn {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout<Message>(
        &self,
        children: &mut [Element<'_, Message>],
        trees: &mut [Tree],
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // A minimum beats the maximum; the space there is beats both.
        let max = Size::new(
            self.max_width.max(self.min_width).min(limits.max().width),
            limits.max().height,
        );

        let natural = children
            .iter_mut()
            .zip(trees.iter_mut())
            .map(|(child, tree)| natural_layout(child, tree, renderer, max).size().width)
            .fold(0.0, f32::max);
        let width = natural
            .max(self.min_width)
            .max(limits.min().width)
            .min(max.width);

        // Allocate: every child at that width.
        let allocate = layout::Limits::new(Size::new(width, 0.0), Size::new(width, max.height));
        let mut y = 0.0;
        let nodes = children
            .iter_mut()
            .zip(trees.iter_mut())
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
}
