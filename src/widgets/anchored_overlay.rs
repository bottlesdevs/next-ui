use iced::{
    Element, Event, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    keyboard::{self, key},
    touch,
};

use super::{
    control::focus_first_descendant,
    spacing,
    surface::{Kind as SurfaceKind, scoped_overlay},
};

#[derive(Clone, Copy)]
pub(super) enum Width {
    MatchAnchor,
    NaturalAtLeastAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dismissal {
    OutsidePress,
    Escape,
    ContentMessage,
}

pub(super) struct PanelContent<'a, Message> {
    children: Vec<Element<'a, Message>>,
}

impl<'a, Message> PanelContent<'a, Message> {
    pub(super) fn new(
        body: impl Into<Element<'a, Message>>,
        footer: Option<Element<'a, Message>>,
    ) -> Self {
        Self {
            children: std::iter::once(body.into()).chain(footer).collect(),
        }
    }

    pub(super) fn tree(&self) -> Tree {
        let mut tree = Tree::empty();
        tree.children = self.children.iter().map(Tree::new).collect();
        tree
    }

    pub(super) fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }
}

/// The common body/footer plumbing for content anchored to another widget.
pub(super) struct AnchoredOverlay<'a, 'b, Message>
where
    'b: 'a,
{
    anchor: Rectangle,
    viewport: Rectangle,
    content: &'a mut PanelContent<'b, Message>,
    tree: &'a mut Tree,
    width: Width,
    viewport_inset: f32,
    focus_content: Option<&'a mut bool>,
    on_dismiss: Box<dyn FnMut(Dismissal) -> bool + 'a>,
}

impl<'a, 'b, Message> AnchoredOverlay<'a, 'b, Message>
where
    'b: 'a,
{
    pub(super) fn new(
        anchor: Rectangle,
        viewport: Rectangle,
        content: &'a mut PanelContent<'b, Message>,
        tree: &'a mut Tree,
        width: Width,
        viewport_inset: f32,
        focus_content: Option<&'a mut bool>,
        on_dismiss: impl FnMut(Dismissal) -> bool + 'a,
    ) -> Self {
        Self {
            anchor,
            viewport,
            content,
            tree,
            width,
            viewport_inset,
            focus_content,
            on_dismiss: Box::new(on_dismiss),
        }
    }

    fn dismiss(&mut self, reason: Dismissal) -> bool {
        let capture = (self.on_dismiss)(reason);

        if capture && let Some(focus_content) = &mut self.focus_content {
            **focus_content = false;
        }

        capture
    }
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    min_width: f32,
    max_width: f32,
    below: f32,
    above: f32,
    inset: f32,
    viewport: Rectangle,
}

fn geometry(width: Width, anchor: Rectangle, viewport: Rectangle, inset: f32) -> Geometry {
    let gap = spacing::XS;
    let max_width = (viewport.width - inset * 2.0).max(0.0);
    let min_width = anchor.width.min(max_width);
    let below = viewport.y + viewport.height - inset - (anchor.y + anchor.height + gap);
    let above = anchor.y - gap - (viewport.y + inset);

    Geometry {
        min_width,
        max_width: if matches!(width, Width::MatchAnchor) {
            min_width
        } else {
            max_width
        },
        below,
        above,
        inset,
        viewport,
    }
}

fn dismissal(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> Option<Dismissal> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => (!cursor.is_over(bounds)
            && cursor.position().is_some())
        .then_some(Dismissal::OutsidePress),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => {
            (!bounds.contains(*position)).then_some(Dismissal::OutsidePress)
        }
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        }) => Some(Dismissal::Escape),
        _ => None,
    }
}

fn panel_width(geometry: Geometry, measured: f32) -> f32 {
    measured.clamp(geometry.min_width, geometry.max_width)
}

impl Geometry {
    fn position(self, anchor: Point, target_height: f32, width: f32, height: f32) -> Point {
        let gap = spacing::XS;
        let open_below = self.below >= height || self.below >= self.above;
        let y = if open_below {
            anchor.y + target_height + gap
        } else {
            anchor.y - height - gap
        };
        let left = self.viewport.x + self.inset;
        let top = self.viewport.y + self.inset;
        let x = anchor.x.clamp(
            left,
            (self.viewport.x + self.viewport.width - width - self.inset).max(left),
        );
        let y = y.clamp(
            top,
            (self.viewport.y + self.viewport.height - height - self.inset).max(top),
        );

        Point::new(x, y)
    }
}

impl<Message: Clone> iced::advanced::Overlay<Message, Theme, iced::Renderer>
    for AnchoredOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let bounds = Rectangle::with_size(bounds);
        let viewport = self.viewport.intersection(&bounds).unwrap_or(bounds);
        let geometry = geometry(self.width, self.anchor, viewport, self.viewport_inset);
        let max_height = geometry.below.max(geometry.above).max(0.0);
        let (body, footer) = self
            .content
            .children
            .split_first_mut()
            .expect("anchored panel body");
        let (body_tree, footer_trees) = self
            .tree
            .children
            .split_first_mut()
            .expect("anchored panel body tree");

        let width = if matches!(self.width, Width::NaturalAtLeastAnchor) {
            let probe_limits = layout::Limits::with_compression(
                Size::ZERO,
                Size::new(geometry.max_width, max_height),
                Size::new(true, false),
            );
            let footer_width =
                footer
                    .first_mut()
                    .zip(footer_trees.first_mut())
                    .map_or(0.0, |(footer, tree)| {
                        footer
                            .as_widget_mut()
                            .layout(tree, renderer, &probe_limits)
                            .size()
                            .width
                    });
            let body_width = body
                .as_widget_mut()
                .layout(body_tree, renderer, &probe_limits)
                .size()
                .width;

            panel_width(geometry, body_width.max(footer_width))
        } else {
            geometry.min_width
        };
        let limits = layout::Limits::new(Size::new(width, 0.0), Size::new(width, max_height));
        let footer = footer
            .first_mut()
            .zip(footer_trees.first_mut())
            .map(|(footer, tree)| footer.as_widget_mut().layout(tree, renderer, &limits));
        let footer_height = footer.as_ref().map_or(0.0, |node| node.size().height);
        let body_limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, (max_height - footer_height).max(0.0)),
        );
        let body = body
            .as_widget_mut()
            .layout(body_tree, renderer, &body_limits);
        let body_height = body.size().height;
        let height = (body_height + footer_height).min(max_height);
        let mut children = vec![body];

        if let Some(footer) = footer {
            children.push(footer.move_to(Point::new(0.0, body_height)));
        }

        layout::Node::with_children(Size::new(width, height), children).move_to(geometry.position(
            self.anchor.position(),
            self.anchor.height,
            width,
            height,
        ))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Some(reason) = dismissal(event, cursor, layout.bounds()) {
            if self.dismiss(reason) {
                shell.capture_event();
                shell.request_redraw();
            }

            return;
        }

        if let Some(focus_content) = &mut self.focus_content
            && **focus_content
        {
            if let (Some(body), Some(body_tree), Some(body_layout)) = (
                self.content.children.first_mut(),
                self.tree.children.first_mut(),
                layout.children().next(),
            ) {
                focus_first_descendant(body, body_tree, body_layout, renderer);
            }

            **focus_content = false;
            shell.request_redraw();
        }

        let mut messages = Vec::new();
        let mut panel_shell = Shell::new(&mut messages);

        for ((child, tree), child_layout) in self
            .content
            .children
            .iter_mut()
            .zip(&mut self.tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                &mut panel_shell,
                &self.viewport,
            );
        }

        if !panel_shell.is_empty() && self.dismiss(Dismissal::ContentMessage) {
            panel_shell.capture_event();
            panel_shell.request_redraw();
        }

        shell.merge(panel_shell, std::convert::identity);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .children
            .iter()
            .zip(&self.tree.children)
            .zip(layout.children())
            .map(|((child, tree), child_layout)| {
                child.as_widget().mouse_interaction(
                    tree,
                    child_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let kind = SurfaceKind::Overlay;
        let scoped_theme = kind.draw_background(renderer, theme, layout.bounds());

        let style = renderer::Style {
            text_color: scoped_theme.palette().text,
        };

        for ((child, tree), child_layout) in self
            .content
            .children
            .iter()
            .zip(&self.tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                tree,
                renderer,
                &scoped_theme,
                &style,
                child_layout,
                cursor,
                &self.viewport,
            );
        }
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, tree), child_layout) in self
                .content
                .children
                .iter_mut()
                .zip(&mut self.tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(tree, child_layout, renderer, operation);
            }
        });
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.content.children,
            self.tree,
            layout,
            renderer,
            &self.viewport,
            Vector::ZERO,
        )
        .map(|content| scoped_overlay(SurfaceKind::Overlay, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_width_uses_anchor_floor_and_viewport_cap() {
        let geometry = geometry(
            Width::NaturalAtLeastAnchor,
            Rectangle::new(Point::new(20.0, 20.0), Size::new(80.0, 32.0)),
            Rectangle::new(Point::new(10.0, 10.0), Size::new(300.0, 200.0)),
            spacing::SM,
        );

        assert_eq!(panel_width(geometry, 40.0), 80.0);
        assert_eq!(panel_width(geometry, 140.0), 140.0);
        assert_eq!(panel_width(geometry, 500.0), 276.0);
    }

    #[test]
    fn matched_width_position_is_clamped_to_the_viewport() {
        let anchor = Rectangle::new(Point::new(280.0, 50.0), Size::new(100.0, 30.0));
        let geometry = geometry(
            Width::MatchAnchor,
            anchor,
            Rectangle::new(Point::new(20.0, 10.0), Size::new(300.0, 200.0)),
            0.0,
        );

        assert_eq!(
            geometry
                .position(anchor.position(), anchor.height, anchor.width, 50.0)
                .x,
            220.0
        );
    }

    #[test]
    fn placement_prefers_below_when_content_fits() {
        let anchor = Rectangle::new(Point::new(40.0, 110.0), Size::new(80.0, 30.0));
        let geometry = geometry(
            Width::NaturalAtLeastAnchor,
            anchor,
            Rectangle::with_size(Size::new(200.0, 200.0)),
            spacing::SM,
        );

        assert_eq!(
            geometry
                .position(anchor.position(), anchor.height, 80.0, 30.0)
                .y,
            146.0
        );
        assert_eq!(
            geometry
                .position(anchor.position(), anchor.height, 80.0, 60.0)
                .y,
            44.0
        );
    }

    #[test]
    fn touch_outside_panel_requests_dismissal() {
        let event = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(0),
            position: Point::new(5.0, 5.0),
        });

        assert_eq!(
            dismissal(
                &event,
                mouse::Cursor::Unavailable,
                Rectangle::new(Point::new(10.0, 10.0), Size::new(100.0, 100.0)),
            ),
            Some(Dismissal::OutsidePress)
        );
    }
}
