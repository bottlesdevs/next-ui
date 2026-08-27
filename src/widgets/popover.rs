use iced::{
    Element, Event, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{column, container, scrollable, svg, text::Fragment, text::IntoFragment, tooltip},
};

use crate::icons::Icon;

use super::{
    anchored_panel::{
        AnchoredPanel, PanelContent, footer as panel_footer, row_content,
        row_style as panel_row_style,
    },
    button::{Button, ButtonKind},
    control::Control,
    spacing,
};

/// A single row inside a [`Popover`]'s panel: either a plain navigable row, or one with a
/// trailing action button (e.g. "Install"). Matches the mixed row kinds seen in the
/// account-linking / storefront-picker mockups.
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

/// A trigger element that opens a floating, anchored panel of [`PopoverItem`]s with an
/// optional pinned footer row, used for the account-linking/storefront-picker and profile
/// switcher flows.
pub struct Popover<'a, Message> {
    trigger: Element<'a, ()>,
    items: Vec<PopoverItem<'a, Message>>,
    footer: Option<(&'a str, Message)>,
}

impl<'a, Message> Popover<'a, Message> {
    pub fn new(trigger: impl Into<Element<'a, ()>>) -> Self {
        Self {
            trigger: trigger.into(),
            items: Vec::new(),
            footer: None,
        }
    }

    pub fn add(mut self, item: PopoverItem<'a, Message>) -> Self {
        self.items.push(item);
        self
    }

    pub fn footer(mut self, label: &'a str, on_press: Message) -> Self {
        self.footer = Some((label, on_press));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let body: Element<'a, Message> = if popover.items.is_empty() {
            column![].into()
        } else {
            let mut rows = column![];

            for item in popover.items {
                rows = rows.push(item_row(item));
            }

            container(rows).padding(spacing::MD).into()
        };

        let footer = popover
            .footer
            .map(|(label, message)| panel_footer(label, message));

        Element::new(PopoverWidget {
            trigger: popover.trigger,
            panel: PanelContent::new(scrollable(body), footer),
        })
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

    let row: Element<'a, Message> = Control::new(content)
        .padding([spacing::XS, spacing::MD])
        .on_press_maybe(item.on_select)
        .selected(item.selected)
        .style(panel_row_style)
        .into();

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
pub(super) struct State {
    pub(super) open: bool,
    pub(super) focus_panel: bool,
    pub(super) focus_trigger: bool,
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
            state.focus_panel = state.open;
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

        Some(overlay::Element::new(Box::new(AnchoredPanel::popover(
            bounds.position() + translation,
            bounds.height,
            bounds.width,
            *viewport,
            &mut self.panel,
            &mut children[1],
            state,
        ))))
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    crate::theme::surface(theme.extended_palette().background.strongest)
}
