use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Point, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    widget::{button, column, container, row, scrollable, svg, text, tooltip},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    pressable::{Pressable, Status},
    spacing,
    text::TextExt as _,
};

/// A single row inside a [`Popover`]'s panel: either a plain navigable row, or one with a
/// trailing action button (e.g. "Install"). Matches the mixed row kinds seen in the
/// account-linking / storefront-picker mockups.
pub struct PopoverItem<'a, Message> {
    title: &'a str,
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
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
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
    trigger: Element<'a, Message>,
    items: Vec<PopoverItem<'a, Message>>,
    footer: Option<(&'a str, Message)>,
    open: bool,
    on_dismiss: Option<Message>,
}

impl<'a, Message> Popover<'a, Message> {
    pub fn new(trigger: impl Into<Element<'a, Message>>, open: bool) -> Self {
        Self {
            trigger: trigger.into(),
            items: Vec::new(),
            footer: None,
            open,
            on_dismiss: None,
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

    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let body: Element<'a, Message> = if popover.items.is_empty() {
            column![].into()
        } else {
            let mut rows = column![].width(Fill);

            for item in popover.items {
                rows = rows.push(item_row(item));
            }

            container(rows).width(Fill).padding(spacing::MD).into()
        };

        let footer = popover.footer.map(|(label, message)| {
            Pressable::new(
                row![text(label), Icon::Arrow.rotated(std::f32::consts::PI)]
                    .spacing(spacing::SM)
                    .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding(spacing::MD)
            .on_press(message)
            .style(footer_style)
            .into()
        });

        Element::new(PopoverWidget {
            trigger: popover.trigger,
            panel_body: scrollable(body).width(Fill).into(),
            panel_footer: footer,
            open: popover.open,
            on_dismiss: popover.on_dismiss,
        })
    }
}

fn item_row<'a, Message: Clone + 'a>(item: PopoverItem<'a, Message>) -> Element<'a, Message> {
    let mut labels = column![text(item.title).label()].spacing(spacing::XS);

    if let Some(subtitle) = item.subtitle {
        labels = labels.push(text(subtitle).detail().muted());
    }

    let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = item.icon {
        content = content.push(
            svg(icon.handle())
                .width(20)
                .height(20)
                .content_fit(iced::ContentFit::Contain),
        );
    }

    content = content
        .push(labels)
        .push(iced::widget::Space::new().width(Fill));

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

    let row: Element<'a, Message> = Pressable::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press_maybe(item.on_select)
        .style(row_style)
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
    trigger: Element<'a, Message>,
    panel_body: Element<'a, Message>,
    panel_footer: Option<Element<'a, Message>>,
    open: bool,
    on_dismiss: Option<Message>,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for PopoverWidget<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn children(&self) -> Vec<Tree> {
        let mut children = vec![Tree::new(&self.trigger), Tree::new(&self.panel_body)];

        if let Some(footer) = &self.panel_footer {
            children.push(Tree::new(footer));
        }

        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut children: Vec<&dyn Widget<Message, Theme, iced::Renderer>> =
            vec![self.trigger.as_widget(), self.panel_body.as_widget()];

        if let Some(footer) = &self.panel_footer {
            children.push(footer.as_widget());
        }

        tree.diff_children(&children);
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
        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
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
        _renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        if !self.open {
            return None;
        }

        let bounds = layout.bounds();

        let (_, panel_trees) = tree.children.split_at_mut(1);
        let (body_trees, footer_trees) = panel_trees.split_at_mut(1);

        Some(overlay::Element::new(Box::new(PopoverOverlay {
            position: bounds.position() + translation,
            target_height: bounds.height,
            width: bounds.width.max(240.0),
            viewport: *viewport,
            body: &mut self.panel_body,
            body_tree: &mut body_trees[0],
            footer: self.panel_footer.as_mut().zip(footer_trees.first_mut()),
            on_dismiss: self.on_dismiss.clone(),
        })))
    }
}

struct PopoverOverlay<'a, 'b, Message>
where
    'b: 'a,
{
    position: Point,
    target_height: f32,
    width: f32,
    viewport: Rectangle,
    body: &'a mut Element<'b, Message>,
    body_tree: &'a mut Tree,
    footer: Option<(&'a mut Element<'b, Message>, &'a mut Tree)>,
    on_dismiss: Option<Message>,
}

impl<Message: Clone> iced::advanced::Overlay<Message, Theme, iced::Renderer>
    for PopoverOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let gap = spacing::XS;
        let viewport_padding = spacing::SM;

        // Keep the popover away from the window edges.
        let available_width = (bounds.width - viewport_padding * 2.0).max(0.0);

        let width = self.width.min(available_width);

        // Horizontal placement.
        let x = self.position.x.clamp(
            viewport_padding,
            (bounds.width - width - viewport_padding).max(viewport_padding),
        );

        // Calculate available space above and below the trigger.
        let below = bounds.height - (self.position.y + self.target_height + gap) - viewport_padding;

        let above = self.position.y - gap - viewport_padding;

        // If neither side can fit the whole popover, use whichever side has
        // more space and let the scrollable body handle the constrained height.
        let open_below = below >= above;

        let max_height = below.max(above).max(0.0);

        let limits = layout::Limits::new(Size::new(width, 0.0), Size::new(width, max_height));

        let footer = self
            .footer
            .as_mut()
            .map(|(footer, tree)| footer.as_widget_mut().layout(tree, renderer, &limits));

        let footer_height = footer.as_ref().map_or(0.0, |node| node.size().height);

        let body_limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, (max_height - footer_height).max(0.0)),
        );

        let body = self
            .body
            .as_widget_mut()
            .layout(self.body_tree, renderer, &body_limits);

        let body_height = body.size().height;
        let height = (body_height + footer_height).min(max_height);

        let y = if open_below {
            self.position.y + self.target_height + gap
        } else {
            self.position.y - height - gap
        };

        // Keep the popover away from the top/bottom window edges.
        let y = y.clamp(
            viewport_padding,
            (bounds.height - height - viewport_padding).max(viewport_padding),
        );

        let mut children = vec![body];

        if let Some(footer) = footer {
            children.push(footer.move_to(Point::new(0.0, body_height)));
        }

        layout::Node::with_children(Size::new(width, height), children).move_to(Point::new(x, y))
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
        let dismiss = match event {
            Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                !cursor.is_over(layout.bounds()) && cursor.position().is_some()
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) => true,
            _ => false,
        };

        if dismiss {
            if let Some(message) = self.on_dismiss.clone() {
                shell.publish(message);
                shell.capture_event();
            }

            return;
        }

        let mut children = layout.children();

        self.body.as_widget_mut().update(
            self.body_tree,
            event,
            children.next().expect("popover panel body layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );

        if let Some(((footer, tree), footer_layout)) = self.footer.as_mut().zip(children.next()) {
            footer.as_widget_mut().update(
                tree,
                event,
                footer_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                &self.viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut children = layout.children();

        let body = self.body.as_widget().mouse_interaction(
            self.body_tree,
            children.next().expect("popover panel body layout"),
            cursor,
            &self.viewport,
            renderer,
        );

        self.footer
            .as_ref()
            .zip(children.next())
            .map_or(body, |((footer, tree), footer_layout)| {
                body.max(footer.as_widget().mouse_interaction(
                    tree,
                    footer_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                ))
            })
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: Border::default().rounded(12),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(theme.extended_palette().background.neutral.color),
        );

        let mut children = layout.children();

        self.body.as_widget().draw(
            self.body_tree,
            renderer,
            theme,
            style,
            children.next().expect("popover panel body layout"),
            cursor,
            &self.viewport,
        );

        if let Some(((footer, tree), footer_layout)) = self.footer.as_ref().zip(children.next()) {
            footer.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                footer_layout,
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
            let mut children = layout.children();

            self.body.as_widget_mut().operate(
                self.body_tree,
                children.next().expect("popover panel body layout"),
                renderer,
                operation,
            );

            if let Some(((footer, tree), footer_layout)) = self.footer.as_mut().zip(children.next())
            {
                footer
                    .as_widget_mut()
                    .operate(tree, footer_layout, renderer, operation);
            }
        });
    }
}

fn row_style(theme: &Theme, status: Status) -> button::Style {
    let highlighted = matches!(status, Status::Hovered | Status::Pressed | Status::Focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(10),
        ..button::Style::default()
    }
}

fn footer_style(theme: &Theme, status: Status) -> button::Style {
    let colors = if matches!(status, Status::Hovered | Status::Pressed) {
        theme.extended_palette().background.strongest
    } else {
        theme.extended_palette().background.stronger
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: theme.extended_palette().secondary.weak.text,
        border: Border::default().rounded(iced::border::bottom(12)),
        ..button::Style::default()
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.strongest.color,
        )),
        text_color: Some(theme.palette().text),
        border: Border::default().rounded(8),
        ..container::Style::default()
    }
}
