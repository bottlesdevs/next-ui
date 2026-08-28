use iced::{
    Event, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    keyboard::{self, key},
    touch,
};

use super::{control::focus_first_descendant, spacing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dismissal {
    OutsidePress,
    Escape,
    ContentMessage,
}

/// Positions arbitrary content relative to another widget.
pub(super) struct AnchoredOverlay<'a, Message> {
    anchor: Rectangle,
    viewport: Rectangle,
    content: &'a mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &'a mut Tree,
    viewport_inset: f32,
    focus_content: Option<&'a mut bool>,
    on_dismiss: Box<dyn FnMut(Dismissal) -> bool + 'a>,
}

impl<'a, Message> AnchoredOverlay<'a, Message> {
    pub(super) fn new(
        anchor: Rectangle,
        viewport: Rectangle,
        content: &'a mut dyn Widget<Message, Theme, iced::Renderer>,
        tree: &'a mut Tree,
        viewport_inset: f32,
        focus_content: Option<&'a mut bool>,
        on_dismiss: impl FnMut(Dismissal) -> bool + 'a,
    ) -> Self {
        Self {
            anchor,
            viewport,
            content,
            tree,
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
    max_width: f32,
    below: f32,
    above: f32,
    inset: f32,
    viewport: Rectangle,
}

fn geometry(anchor: Rectangle, viewport: Rectangle, inset: f32) -> Geometry {
    let gap = spacing::XS;
    let max_width = (viewport.width - inset * 2.0).max(0.0);
    let below = viewport.y + viewport.height - inset - (anchor.y + anchor.height + gap);
    let above = anchor.y - gap - (viewport.y + inset);

    Geometry {
        max_width,
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
    for AnchoredOverlay<'_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let bounds = Rectangle::with_size(bounds);
        let viewport = self.viewport.intersection(&bounds).unwrap_or(bounds);
        let geometry = geometry(self.anchor, viewport, self.viewport_inset);
        let max_height = geometry.below.max(geometry.above).max(0.0);
        let content = self.content.layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(geometry.max_width, max_height)),
        );
        let size = content.size();

        content.move_to(geometry.position(
            self.anchor.position(),
            self.anchor.height,
            size.width,
            size.height,
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
            focus_first_descendant(self.content, self.tree, layout, renderer);

            **focus_content = false;
            shell.request_redraw();
        }

        let mut messages = Vec::new();
        let mut panel_shell = Shell::new(&mut messages);

        self.content.update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut panel_shell,
            &self.viewport,
        );

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
            .mouse_interaction(self.tree, layout, cursor, &self.viewport, renderer)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &self.viewport,
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.operate(self.tree, layout, renderer, operation);
        });
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content
            .overlay(self.tree, layout, renderer, &self.viewport, Vector::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_width_accounts_for_the_viewport_inset() {
        let requested = geometry(
            Rectangle::new(Point::new(20.0, 20.0), Size::new(80.0, 32.0)),
            Rectangle::new(Point::new(10.0, 10.0), Size::new(300.0, 200.0)),
            spacing::SM,
        );

        assert_eq!(requested.max_width, 276.0);
    }

    #[test]
    fn matched_width_position_is_clamped_to_the_viewport() {
        let anchor = Rectangle::new(Point::new(280.0, 50.0), Size::new(100.0, 30.0));
        let geometry = geometry(
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
