//! Shared widths, like `GtkSizeGroup`: every [`Shared`] in a [`SizeGroup`]
//! with the same key is as wide as the widest of them, so what sits beside
//! them lines up across rows whatever each one holds.

use iced::{
    Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};

/// Names which members of a group share a width.
pub type Key = &'static str;

/// The scope whose [`Shared`] members share widths. A group nested inside
/// it is a scope of its own.
pub struct SizeGroup<'a, Message> {
    content: Element<'a, Message>,
}

/// Content that takes the width of the widest member with its key in the
/// nearest [`SizeGroup`], aligned to that width's end. Outside any group it
/// is as wide as its content.
pub struct Shared<'a, Message> {
    key: Key,
    content: Element<'a, Message>,
}

struct Scope;

#[derive(Default)]
struct Member {
    key: Key,
    /// The content's own width at the last layout.
    natural: f32,
    /// The width the group last asked for.
    shared: f32,
}

impl<'a, Message> SizeGroup<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<'a, Message> Shared<'a, Message> {
    pub fn new(key: Key, content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            key,
            content: content.into(),
        }
    }
}

/// Calls `visit` on every member in `tree`, not looking inside nested groups.
fn members(tree: &mut Tree, visit: &mut impl FnMut(&mut Member)) {
    for child in &mut tree.children {
        if child.tag == tree::Tag::of::<Scope>() {
            continue;
        }
        if child.tag == tree::Tag::of::<Member>() {
            visit(child.state.downcast_mut::<Member>());
        }
        members(child, visit);
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for SizeGroup<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Scope>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Scope)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // Measure every member at its own width, then lay out again with
        // each at the widest of its key.
        members(tree, &mut |member| member.shared = 0.0);
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        let mut widest: Vec<(Key, f32)> = Vec::new();
        members(
            tree,
            &mut |member| match widest.iter_mut().find(|(key, _)| *key == member.key) {
                Some((_, width)) => *width = width.max(member.natural),
                None => widest.push((member.key, member.natural)),
            },
        );
        if widest.is_empty() {
            return layout::Node::with_children(node.size(), vec![node]);
        }
        members(tree, &mut |member| {
            member.shared = widest
                .iter()
                .find(|(key, _)| *key == member.key)
                .map_or(0.0, |(_, width)| *width);
        });

        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(node.size(), vec![node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operate(&mut self.content, tree, layout, renderer, operation);
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
        update(
            &mut self.content,
            tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
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
        mouse_interaction(&self.content, tree, layout, cursor, viewport, renderer)
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
        draw(
            &self.content,
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        overlay(
            &mut self.content,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Shared<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Member>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Member {
            key: self.key,
            ..Member::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.state.downcast_mut::<Member>().key = self.key;
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        let size = self.content.as_widget().size();
        Size::new(Length::Shrink, size.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let member = tree.state.downcast_mut::<Member>();
        let natural = node.size().width;
        member.natural = natural;
        let width = natural.max(member.shared).min(limits.max().width);
        let height = node.size().height;

        layout::Node::with_children(
            Size::new(width, height),
            vec![node.move_to(Point::new(width - natural, 0.0))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operate(&mut self.content, tree, layout, renderer, operation);
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
        update(
            &mut self.content,
            tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
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
        mouse_interaction(&self.content, tree, layout, cursor, viewport, renderer)
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
        draw(
            &self.content,
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        overlay(
            &mut self.content,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

// Both widgets hold one child as the only node under their own.

fn child<'b>(layout: Layout<'b>) -> Layout<'b> {
    layout
        .children()
        .next()
        .expect("a size group node has its child")
}

fn operate<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    operation: &mut dyn Operation,
) {
    content
        .as_widget_mut()
        .operate(&mut tree.children[0], child(layout), renderer, operation);
}

#[allow(clippy::too_many_arguments)]
fn update<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &iced::Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle,
) {
    content.as_widget_mut().update(
        &mut tree.children[0],
        event,
        child(layout),
        cursor,
        renderer,
        clipboard,
        shell,
        viewport,
    );
}

fn mouse_interaction<Message>(
    content: &Element<'_, Message>,
    tree: &Tree,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    renderer: &iced::Renderer,
) -> mouse::Interaction {
    content.as_widget().mouse_interaction(
        &tree.children[0],
        child(layout),
        cursor,
        viewport,
        renderer,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw<Message>(
    content: &Element<'_, Message>,
    tree: &Tree,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    style: &renderer::Style,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
) {
    content.as_widget().draw(
        &tree.children[0],
        renderer,
        theme,
        style,
        child(layout),
        cursor,
        viewport,
    );
}

fn overlay<'b, Message>(
    content: &'b mut Element<'_, Message>,
    tree: &'b mut Tree,
    layout: Layout<'b>,
    renderer: &iced::Renderer,
    viewport: &Rectangle,
    translation: Vector,
) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
    content.as_widget_mut().overlay(
        &mut tree.children[0],
        child(layout),
        renderer,
        viewport,
        translation,
    )
}

impl<'a, Message: 'a> From<SizeGroup<'a, Message>> for Element<'a, Message> {
    fn from(group: SizeGroup<'a, Message>) -> Self {
        Element::new(group)
    }
}

impl<'a, Message: 'a> From<Shared<'a, Message>> for Element<'a, Message> {
    fn from(shared: Shared<'a, Message>) -> Self {
        Element::new(shared)
    }
}
