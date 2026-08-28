use iced::{
    Element, Event, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{container, scrollable, svg, text::Fragment, text::IntoFragment, tooltip},
};

use crate::icons::Icon;

use super::{
    anchored_overlay::{AnchoredOverlay, PanelContent, Width},
    button::{Button, ButtonKind},
    menu::{MenuRows, item as menu_item, row_content},
    spacing,
};

/// A trigger that opens an anchored menu of [`PopoverItem`]s.
pub struct Popover<'a, Message> {
    trigger: Element<'a, ()>,
    items: Vec<PopoverItem<'a, Message>>,
}

impl<'a, Message> Popover<'a, Message> {
    pub fn new(trigger: impl Into<Element<'a, ()>>) -> Self {
        Self {
            trigger: trigger.into(),
            items: Vec::new(),
        }
    }

    pub fn item(mut self, item: PopoverItem<'a, Message>) -> Self {
        self.items.push(item);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let rows = popover.items.into_iter().map(item_row);

        Element::new(PopoverWidget {
            trigger: popover.trigger,
            panel: PanelContent::new(scrollable(MenuRows::new(rows)), None),
        })
    }
}

/// A row in a [`Popover`], optionally containing a trailing action.
pub struct PopoverItem<'a, Message> {
    title: Fragment<'a>,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
    on_select: Option<Message>,
    action: Option<(&'a str, Message)>,
    /// Shows a disabled action button (still visible, greyed out and
    /// non-interactive) instead of omitting it entirely — used to explain
    /// *why* an action isn't available (e.g. "Taken") rather than just
    /// hiding it.
    disabled_action: Option<&'a str>,
    tooltip: Option<Element<'a, Message>>,
    selected: bool,
}

impl<'a, Message> PopoverItem<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: None,
            icon: None,
            on_select: None,
            action: None,
            disabled_action: None,
            tooltip: None,
            selected: false,
        }
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_select(mut self, message: Message) -> Self {
        self.on_select = Some(message);
        self
    }

    pub fn action(mut self, label: &'a str, message: Message) -> Self {
        self.action = Some((label, message));
        self
    }

    pub fn disabled_action(mut self, label: &'a str) -> Self {
        self.disabled_action = Some(label);
        self
    }

    /// Wraps the row in a tooltip shown on hover — e.g. explaining a
    /// [`disabled_action`](Self::disabled_action).
    pub fn tooltip(mut self, tooltip: impl Into<Element<'a, Message>>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Marks this row as the currently active choice — shows a trailing
    /// checkmark instead of relying on the caller to encode that in the
    /// title/subtitle text.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

fn item_row<'a, Message: Clone + 'a>(item: PopoverItem<'a, Message>) -> Element<'a, Message> {
    let mut content = row_content(item.title, item.subtitle, item.icon);

    if item.selected {
        content = content.push(
            svg(Icon::Checkmark.handle())
                .width(16)
                .height(16)
                .content_fit(iced::ContentFit::Contain),
        );
    }

    if let Some((label, message)) = item.action {
        content = content.push(
            Button::new(label)
                .kind(ButtonKind::Surface)
                .on_press(message),
        );
    } else if let Some(label) = item.disabled_action {
        content = content.push(Button::new(label).kind(ButtonKind::Surface));
    }

    let row = menu_item(content, item.on_select, item.selected, || false);

    match item.tooltip {
        Some(tip) => tooltip(row, tip, tooltip::Position::Top)
            .style(tooltip_style)
            .padding(spacing::SM)
            .into(),
        None => row,
    }
}

struct PopoverWidget<'a, Message> {
    trigger: Element<'a, ()>,
    panel: PanelContent<'a, Message>,
}

fn unexpected_trigger_overlay_message<Message>((): ()) -> Message {
    unreachable!("popover trigger overlays are presentational")
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    focus_content: bool,
    focus_trigger: bool,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for PopoverWidget<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.trigger), self.panel.tree()]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() != 2 {
            tree.children = self.children();
            return;
        }

        tree.children[0].diff(&self.trigger);
        self.panel.diff(&mut tree.children[1]);
    }

    fn size(&self) -> Size<Length> {
        self.trigger.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.trigger
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
        self.trigger
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

        if state.focus_trigger {
            super::control::focus_first_descendant(
                &mut self.trigger,
                &mut tree.children[0],
                layout,
                renderer,
            );
            state.focus_trigger = false;
        }

        let mut messages = Vec::new();
        let mut trigger_shell = Shell::new(&mut messages);

        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut trigger_shell,
            viewport,
        );

        if trigger_shell.is_empty() {
            shell.merge(trigger_shell, |()| unreachable!("empty trigger shell"));
        } else {
            state.open = !state.open;
            state.focus_content = state.open && matches!(event, Event::Keyboard(_));
            state.focus_trigger = !state.open;
            shell.capture_event();
            shell.request_redraw();
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
        self.trigger.as_widget().mouse_interaction(
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
        self.trigger.as_widget().draw(
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
        let (state, children) = (&mut tree.state, &mut tree.children);
        let state = state.downcast_mut::<State>();

        let bounds = layout.bounds();

        if !state.open {
            return self
                .trigger
                .as_widget_mut()
                .overlay(&mut children[0], layout, renderer, viewport, translation)
                .map(|content| content.map(&unexpected_trigger_overlay_message::<Message>));
        }

        let State {
            open,
            focus_content,
            focus_trigger,
        } = state;
        let anchor = Rectangle::new(bounds.position() + translation, bounds.size());

        Some(overlay::Element::new(Box::new(AnchoredOverlay::new(
            anchor,
            *viewport,
            &mut self.panel,
            &mut children[1],
            Width::NaturalAtLeastAnchor,
            spacing::SM,
            Some(focus_content),
            move |_| {
                *open = false;
                *focus_trigger = true;
                true
            },
        ))))
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    crate::theme::surface(theme.extended_palette().background.strongest)
}

#[cfg(test)]
mod tests {
    use iced::advanced::widget::Tree;

    use super::*;

    fn popover() -> Element<'static, ()> {
        Popover::new(Button::new("Open").on_press(()))
            .item(PopoverItem::new("Item").on_select(()))
            .into()
    }

    #[test]
    fn rebuilding_retains_local_disclosure_state() {
        let popover = popover();
        let mut tree = Tree::new(&popover);
        tree.state.downcast_mut::<State>().open = true;

        tree.diff(&self::popover());

        assert!(tree.state.downcast_ref::<State>().open);
        assert_eq!(tree.children.len(), 2);
    }
}
