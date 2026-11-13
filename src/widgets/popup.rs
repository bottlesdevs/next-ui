use std::cell::Cell;

use iced::{
    Element, Event, Fill, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{Space, container},
};

use super::{
    anchored_overlay::{Placement, PopupCore, PopupFocus, PopupFocusState},
    focus::{descendant_is_focused, event_cursor, unfocus_descendants},
    spacing,
};

/// A message published from a popup's panel, saying whether it also closes
/// the popup: picking an option does, starting a download beside it doesn't.
#[derive(Debug, Clone)]
pub(super) enum PopupMessage<Message> {
    Emit(Message),
    Close(Message),
}

impl<Message> PopupMessage<Message> {
    pub(super) fn into_inner(self) -> Message {
        match self {
            Self::Emit(message) | Self::Close(message) => message,
        }
    }
}

/// A message published from a popup's anchor.
#[derive(Debug, Clone)]
pub(super) enum AnchorMessage<Message> {
    /// Opens the popup, or closes it if it's open.
    Toggle,
    Emit(Message),
}

/// Publishes the messages of a panel's `panel` shell to `shell`, and returns
/// whether one of them closes the panel.
pub(super) fn forward<Message>(
    shell: &mut Shell<'_, Message>,
    panel: Shell<'_, PopupMessage<Message>>,
) -> bool {
    let close = Cell::new(false);
    shell.merge(panel, |message| match message {
        PopupMessage::Emit(message) => message,
        PopupMessage::Close(message) => {
            close.set(true);
            message
        }
    });
    close.get()
}

/// An anchor widget with a panel floating beside it.
pub(super) struct Popup<'a, Message> {
    anchor: Element<'a, AnchorMessage<Message>>,
    panel: Element<'a, PopupMessage<Message>>,
    trigger: Trigger<'a>,
    focusable: bool,
    enabled: bool,
    placement: Placement,
}

enum Trigger<'a> {
    /// The anchor toggles the popup with [`AnchorMessage::Toggle`].
    Toggle,
    /// The popup follows the anchor's focus.
    Focus { visible: bool, key: &'a str },
}

#[derive(Debug, Default)]
struct State {
    /// For a toggled popup, whether it's open. For one that follows focus,
    /// whether it hasn't been dismissed since the anchor last asked for it.
    open: bool,
    anchor_focused: bool,
    panel_focus: PopupFocusState,
    key: String,
}

impl State {
    fn shown(&self, trigger: &Trigger<'_>) -> bool {
        match trigger {
            Trigger::Toggle => self.open,
            Trigger::Focus { visible, .. } => {
                *visible
                    && self.open
                    && (self.anchor_focused || self.panel_focus != PopupFocusState::None)
            }
        }
    }
}

impl<'a, Message> Popup<'a, Message> {
    /// A popup the anchor opens and closes with [`AnchorMessage::Toggle`].
    pub(super) fn new(
        anchor: impl Into<Element<'a, AnchorMessage<Message>>>,
        panel: impl Into<Element<'a, PopupMessage<Message>>>,
    ) -> Self {
        Self {
            anchor: anchor.into(),
            panel: panel.into(),
            trigger: Trigger::Toggle,
            focusable: true,
            enabled: true,
            placement: Placement::default(),
        }
    }

    /// Shows the panel while the anchor has focus and there is something to
    /// show, e.g. results for a query typed into it. Once dismissed it stays
    /// closed until the anchor is pressed or focused again, or `key` changes.
    pub(super) fn while_focused(mut self, visible: bool, key: &'a str) -> Self {
        self.trigger = Trigger::Focus { visible, key };
        self
    }

    /// Whether the panel has controls for the keyboard to reach. Without
    /// them, opening from the keyboard leaves focus on the anchor, and Tab
    /// moves past the panel.
    pub(super) fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Closes the popup, and keeps it closed, unless `enabled`.
    pub(super) fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Keeps the panel this far from the viewport's edges.
    pub(super) fn viewport_inset(mut self, inset: f32) -> Self {
        self.placement.inset = inset;
        self
    }

    pub(super) fn match_anchor_width(mut self) -> Self {
        self.placement.match_anchor_width = true;
        self
    }

    /// Places the panel flush against the anchor, so the two read as one
    /// surface.
    pub(super) fn attached(mut self) -> Self {
        self.placement.gap = 0.0;
        self
    }
}

/// Whether the toggled popup whose tree is `tree` is open, for a widget that
/// draws around it.
pub(super) fn is_open(tree: &Tree) -> bool {
    tree.state.downcast_ref::<State>().open
}

/// The anchor's tree inside the popup's `tree`.
pub(super) fn anchor(tree: &Tree) -> &Tree {
    &tree.children[0]
}

/// The message of an anchor's own overlay, e.g. a tooltip, which can't
/// toggle the popup.
fn emitted<Message>(message: AnchorMessage<Message>) -> Message {
    match message {
        AnchorMessage::Emit(message) => message,
        AnchorMessage::Toggle => unreachable!("anchor overlays are presentational"),
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Popup<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(match self.trigger {
            Trigger::Toggle => State::default(),
            Trigger::Focus { key, .. } => State {
                open: true,
                key: key.to_owned(),
                ..State::default()
            },
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.anchor), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.anchor);
        tree.children[1].diff(&self.panel);
        let state = tree.state.downcast_mut::<State>();

        if !self.enabled {
            state.open = false;
        }

        if let Trigger::Focus { visible, key } = self.trigger {
            if state.key != key {
                state.key.clear();
                state.key.push_str(key);
                state.open = true;
            }

            if !visible {
                state.panel_focus = PopupFocusState::None;
            }
        }
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.anchor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.anchor
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let state = tree.state.downcast_mut::<State>();
        let anchor = self.anchor.as_widget_mut();

        if let Trigger::Focus { .. } = self.trigger {
            if !state.open
                && matches!(
                    event,
                    Event::Mouse(mouse::Event::ButtonPressed(_))
                        | Event::Touch(touch::Event::FingerPressed { .. })
                )
                && event_cursor(event, cursor).is_over(layout.bounds())
            {
                state.open = true;
            }

            // Tab from the anchor continues into the panel that follows it.
            if state.anchor_focused
                && state.shown(&self.trigger)
                && self.focusable
                && let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Tab),
                    modifiers,
                    repeat: false,
                    ..
                }) = event
                && !modifiers.shift()
            {
                state.panel_focus = PopupFocusState::Requested;
                unfocus_descendants(anchor, &mut tree.children[0], layout, renderer);
                state.anchor_focused = false;
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        let mut messages = Vec::new();
        let mut anchor_shell = Shell::new(&mut messages);
        if shell.is_event_captured() {
            anchor_shell.capture_event();
        }
        anchor.update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut anchor_shell,
            viewport,
        );
        // A toggle has no message to forward, so the anchor's shell is merged
        // by hand.
        if anchor_shell.is_event_captured() {
            shell.capture_event();
        }
        shell.request_redraw_at(anchor_shell.redraw_request());
        shell.request_input_method(anchor_shell.input_method());
        if anchor_shell.is_layout_invalid() {
            shell.invalidate_layout();
        }
        if anchor_shell.are_widgets_invalid() {
            shell.invalidate_widgets();
        }
        let mut toggled = false;
        for message in messages {
            match message {
                AnchorMessage::Emit(message) => shell.publish(message),
                AnchorMessage::Toggle => toggled = true,
            }
        }

        match self.trigger {
            Trigger::Toggle if toggled => {
                if state.open {
                    state.open = false;
                    state.panel_focus = PopupFocusState::None;
                } else {
                    let focus_panel = self.focusable && matches!(event, Event::Keyboard(_));
                    if focus_panel {
                        unfocus_descendants(anchor, &mut tree.children[0], layout, renderer);
                    }
                    state.open = true;
                    state.panel_focus = if focus_panel {
                        PopupFocusState::Requested
                    } else {
                        PopupFocusState::None
                    };
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Trigger::Toggle => {}
            Trigger::Focus { .. } => {
                let focused =
                    descendant_is_focused(anchor, &mut tree.children[0], layout, renderer);
                if focused && !state.anchor_focused {
                    state.open = true;
                }
                state.anchor_focused = focused;
            }
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
        self.anchor.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        self.anchor.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
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
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let (state, children) = (tree.state.downcast_mut::<State>(), &mut tree.children);

        if !state.shown(&self.trigger) {
            return self
                .anchor
                .as_widget_mut()
                .overlay(&mut children[0], layout, renderer, viewport, translation)
                .map(|overlay| overlay.map(&emitted::<Message>));
        }

        let bounds = layout.bounds();
        let State {
            open, panel_focus, ..
        } = state;
        let (anchor_tree, panel_tree) = children.split_at_mut(1);

        Some(overlay::Element::new(Box::new(PopupCore::new(
            Rectangle::new(bounds.position() + translation, bounds.size()),
            *viewport,
            self.panel.as_widget_mut(),
            &mut panel_tree[0],
            self.placement,
            open,
            PopupFocus {
                state: panel_focus,
                anchor: self.anchor.as_widget_mut(),
                anchor_tree: &mut anchor_tree[0],
                anchor_layout: layout,
            },
        ))))
    }
}

/// A hairline between groups of rows in a popup's list.
pub(super) fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    container(
        container(Space::new())
            .width(Fill)
            .height(1)
            .style(|theme| {
                container::Style::default().background(crate::theme::colors(theme).hovered)
            }),
    )
    .padding([spacing::XS, spacing::SM])
    .into()
}

impl<'a, Message: Clone + 'a> From<Popup<'a, Message>> for Element<'a, Message> {
    fn from(popup: Popup<'a, Message>) -> Self {
        Element::new(popup)
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::text;

    use super::*;
    use crate::widgets::button::Button;

    fn popup() -> Element<'static, ()> {
        Popup::new(
            Button::new("Open").on_press(AnchorMessage::Toggle),
            text("Panel"),
        )
        .into()
    }

    #[test]
    fn rebuilding_retains_local_disclosure_state() {
        let popup = popup();
        let mut tree = Tree::new(&popup);
        tree.state.downcast_mut::<State>().open = true;

        tree.diff(self::popup());

        assert!(tree.state.downcast_ref::<State>().open);
        assert_eq!(tree.children.len(), 2);
    }
}
