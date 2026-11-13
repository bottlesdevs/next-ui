use iced::{
    Event, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Id, Operation, Tree, operation},
    },
    keyboard::{self, key},
    touch,
};

use super::{
    focus::{
        cycle_descendant_focus, descendant_is_focused, event_cursor, focus_first_descendant,
        unfocus_descendants,
    },
    popup::{AnchorMessage, PopupMessage, forward},
    spacing,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DismissReason {
    OutsidePress,
    Escape,
    ContentMessage { keyboard: bool },
    FocusLost,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum PopupFocusState {
    #[default]
    None,
    Requested,
    Focused,
}

pub(super) struct PopupFocus<'a, Message> {
    pub(super) state: &'a mut PopupFocusState,
    /// The anchor, which focus goes back to when the keyboard leaves the
    /// panel.
    pub(super) anchor: &'a mut dyn Widget<AnchorMessage<Message>, Theme, iced::Renderer>,
    pub(super) anchor_tree: &'a mut Tree,
    pub(super) anchor_layout: Layout<'a>,
}

/// Where a popup's panel goes relative to its anchor.
#[derive(Debug, Clone, Copy)]
pub(super) struct Placement {
    /// Space kept between the panel and the viewport's edges.
    pub(super) inset: f32,
    pub(super) match_anchor_width: bool,
    /// Space between the anchor and the panel.
    pub(super) gap: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            inset: 0.0,
            match_anchor_width: false,
            gap: spacing::XS,
        }
    }
}

/// The floating half of a [`Popup`]: places its panel by the anchor, moves
/// focus through it and dismisses it.
///
/// [`Popup`]: super::popup::Popup
pub(super) struct PopupCore<'a, Message> {
    anchor: Rectangle,
    viewport: Rectangle,
    content: &'a mut dyn Widget<PopupMessage<Message>, Theme, iced::Renderer>,
    tree: &'a mut Tree,
    placement: Placement,
    open: &'a mut bool,
    focus: PopupFocus<'a, Message>,
}

impl<'a, Message> PopupCore<'a, Message> {
    pub(super) fn new(
        anchor: Rectangle,
        viewport: Rectangle,
        content: &'a mut dyn Widget<PopupMessage<Message>, Theme, iced::Renderer>,
        tree: &'a mut Tree,
        placement: Placement,
        open: &'a mut bool,
        focus: PopupFocus<'a, Message>,
    ) -> Self {
        Self {
            anchor,
            viewport,
            content,
            tree,
            placement,
            open,
            focus,
        }
    }

    /// Closes the popup and returns whether `reason`'s event is used up by
    /// that. Keyboard dismissals of a panel the keyboard is in hand focus
    /// back to the anchor; after the mouse, nothing is left focused.
    fn dismiss(&mut self, reason: DismissReason, renderer: &iced::Renderer) -> bool {
        let in_panel = *self.focus.state != PopupFocusState::None;
        let (captured, restore) = match reason {
            DismissReason::FocusLost | DismissReason::OutsidePress => (false, false),
            DismissReason::ContentMessage { keyboard } => (true, keyboard && in_panel),
            DismissReason::Escape => (true, in_panel),
        };

        *self.open = false;
        *self.focus.state = PopupFocusState::None;
        if restore {
            self.focus_anchor(renderer);
        }

        captured
    }

    fn focus_anchor(&mut self, renderer: &iced::Renderer) {
        focus_first_descendant(
            self.focus.anchor,
            self.focus.anchor_tree,
            self.focus.anchor_layout,
            renderer,
        );
    }

    /// Moves focus to the panel's first control, or along its controls from
    /// the focused one, wrapping around.
    fn focus(&mut self, previous: Option<bool>, layout: Layout<'_>, renderer: &iced::Renderer) {
        match previous {
            None => focus_first_descendant(self.content, self.tree, layout, renderer),
            Some(previous) => {
                cycle_descendant_focus(self.content, self.tree, layout, renderer, previous);
            }
        }

        if descendant_is_focused(self.content, self.tree, layout, renderer) {
            *self.focus.state = PopupFocusState::Focused;
            reveal(self.content, self.tree, layout, renderer);
        } else {
            // Nothing in the panel takes focus, e.g. every item is disabled.
            *self.focus.state = PopupFocusState::None;
            self.focus_anchor(renderer);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    max_width: f32,
    below: f32,
    above: f32,
    inset: f32,
    viewport: Rectangle,
    gap: f32,
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
        gap,
    }
}

/// Whether `event` dismisses something floating or open over `bounds`: a
/// press outside it, or Escape.
pub(super) fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    dismissal(event, cursor, bounds).is_some()
}

fn dismissal(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> Option<DismissReason> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => (!cursor.is_over(bounds)
            && cursor.position().is_some())
        .then_some(DismissReason::OutsidePress),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => {
            (!bounds.contains(*position)).then_some(DismissReason::OutsidePress)
        }
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        }) => Some(DismissReason::Escape),
        _ => None,
    }
}

impl Geometry {
    fn position(self, anchor: Point, target_height: f32, width: f32, height: f32) -> Point {
        let gap = self.gap;
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
    for PopupCore<'_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let bounds = Rectangle::with_size(bounds);
        let viewport = self.viewport.intersection(&bounds).unwrap_or(bounds);
        let mut geometry = geometry(self.anchor, viewport, self.placement.inset);
        geometry.gap = self.placement.gap;
        let max_height = geometry.below.max(geometry.above).max(0.0);
        let width = self.anchor.width.min(geometry.max_width);
        let min = Size::new(
            if self.placement.match_anchor_width {
                width
            } else {
                0.0
            },
            0.0,
        );
        let max = Size::new(
            if self.placement.match_anchor_width {
                width
            } else {
                geometry.max_width
            },
            max_height,
        );
        let content = self
            .content
            .layout(self.tree, renderer, &layout::Limits::new(min, max));
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
        // A press on the anchor is the anchor's to handle: it closes the popup
        // itself on release, and dismissing here too would reopen it.
        let on_anchor = matches!(event, Event::Mouse(_) | Event::Touch(_))
            && event_cursor(event, cursor).is_over(self.anchor);
        if let Some(reason) = dismissal(event, cursor, layout.bounds())
            .filter(|reason| !(on_anchor && *reason == DismissReason::OutsidePress))
        {
            let captured = self.dismiss(reason, renderer);
            unfocus_descendants(self.content, self.tree, layout, renderer);

            if captured {
                shell.capture_event();
            }
            shell.request_redraw();

            return;
        }

        if *self.focus.state == PopupFocusState::Requested {
            self.focus(None, layout, renderer);
            shell.request_redraw();
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Tab),
            modifiers,
            repeat: false,
            ..
        }) = event
            && *self.focus.state == PopupFocusState::Focused
            && descendant_is_focused(self.content, self.tree, layout, renderer)
        {
            self.focus(Some(modifiers.shift()), layout, renderer);
            shell.capture_event();
            shell.request_redraw();
            return;
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

        let pointer_inside = matches!(event, Event::Mouse(_) | Event::Touch(_))
            && event_cursor(event, cursor).is_over(layout.bounds());
        let lost_focus = *self.focus.state == PopupFocusState::Focused
            && !descendant_is_focused(self.content, self.tree, layout, renderer);

        if lost_focus && pointer_inside {
            // A press in the panel takes focus off its controls, not the
            // panel away.
            *self.focus.state = PopupFocusState::None;
        } else if lost_focus && panel_shell.is_empty() && !panel_shell.is_event_captured() {
            self.dismiss(DismissReason::FocusLost, renderer);
            panel_shell.request_redraw();
        }

        if pointer_inside {
            panel_shell.capture_event();
        }

        if forward(shell, panel_shell) {
            let captured = self.dismiss(
                DismissReason::ContentMessage {
                    keyboard: matches!(event, Event::Keyboard(_)),
                },
                renderer,
            );
            unfocus_descendants(self.content, self.tree, layout, renderer);

            if captured {
                shell.capture_event();
            }
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let interaction =
            self.content
                .mouse_interaction(self.tree, layout, cursor, &self.viewport, renderer);

        if interaction == mouse::Interaction::None && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Idle
        } else {
            interaction
        }
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
            .map(|overlay| overlay.map(&PopupMessage::into_inner))
    }
}

/// Scrolls the panel's focused control into view.
fn reveal<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> bool {
    let mut find = FindFocusedBounds { bounds: None };
    content.operate(tree, layout, renderer, &mut find);
    let Some(bounds) = find.bounds else {
        return false;
    };
    let mut reveal = RevealBounds {
        bounds,
        changed: false,
    };
    content.operate(tree, layout, renderer, &mut reveal);
    reveal.changed
}

struct FindFocusedBounds {
    bounds: Option<Rectangle>,
}

impl<Message> Operation<Message> for FindFocusedBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if state.is_focused() {
            self.bounds = Some(bounds);
        }
    }
}

struct RevealBounds {
    bounds: Rectangle,
    changed: bool,
}

impl<Message> Operation<Message> for RevealBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        if !content_bounds.contains(self.bounds.center()) {
            return;
        }

        let target = Rectangle {
            x: self.bounds.x - translation.x,
            y: self.bounds.y - translation.y,
            ..self.bounds
        };
        let x = if target.x < bounds.x {
            target.x - bounds.x
        } else if target.x + target.width > bounds.x + bounds.width {
            target.x + target.width - (bounds.x + bounds.width)
        } else {
            0.0
        };
        let y = if target.y < bounds.y {
            target.y - bounds.y
        } else if target.y + target.height > bounds.y + bounds.height {
            target.y + target.height - (bounds.y + bounds.height)
        } else {
            0.0
        };
        let delta = Vector::new(x, y);

        if delta != Vector::ZERO {
            state.scroll_by(
                operation::scrollable::AbsoluteOffset {
                    x: delta.x,
                    y: delta.y,
                },
                bounds,
                content_bounds,
            );
            self.changed = true;
        }
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
            Some(DismissReason::OutsidePress)
        );
    }
}
