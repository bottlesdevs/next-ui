//! A header that discloses a panel beneath it in place, pushing the content
//! below down, like `AdwExpanderRow`.

use iced::{
    Background, Border, Color, ContentFit, Element, Event, Length, Point, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        svg::Renderer as _,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
};

use super::{
    focusable_action::{ActionState, Interaction},
    list_row,
    popup::{self, PopupMessage},
};

/// The corners of the header and panel, which the bridge between them
/// continues.
const RADIUS: f32 = 6.0;
const CARET: f32 = 20.0;

/// A header that opens and closes a panel laid out under it.
///
/// The header is a row's content without its own fill, with room kept for
/// the caret (see [`ListRow::into_disclosure_content`]). A
/// [`PopupMessage::Close`] from the panel closes it.
///
/// [`ListRow::into_disclosure_content`]: list_row::ListRow::into_disclosure_content
pub(super) struct Disclosure<'a, Message> {
    header: Element<'a, Message>,
    panel: Element<'a, PopupMessage<Message>>,
    enabled: bool,
    gap: f32,
    dismissible: bool,
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    interaction: Interaction,
}

impl<'a, Message> Disclosure<'a, Message> {
    pub(super) fn new(
        header: impl Into<Element<'a, Message>>,
        panel: impl Into<Element<'a, PopupMessage<Message>>>,
    ) -> Self {
        Self {
            header: header.into(),
            panel: panel.into(),
            enabled: true,
            gap: 0.0,
            dismissible: false,
        }
    }

    /// Closes the panel, and keeps it closed, unless `enabled`.
    pub(super) fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the space between the header and the panel.
    pub(super) fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Closes the panel on a press outside it or Escape, as a dropdown does.
    pub(super) fn dismissible(mut self) -> Self {
        self.dismissible = true;
        self
    }

    /// Lays out the header alone, for a container that places the panel
    /// itself, e.g. under a whole line of a grid.
    pub(super) fn layout_header(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.header
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    /// Lays out the open panel `width` wide.
    pub(super) fn layout_panel(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        width: f32,
        max_height: f32,
    ) -> layout::Node {
        self.panel.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::new(width, 0.0), Size::new(width, max_height)),
        )
    }

    pub(super) fn operate_header(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let (header, _) = parts(layout);

        if self.enabled {
            operation.focusable(
                None,
                header.bounds(),
                &mut tree.state.downcast_mut::<State>().interaction,
            );
        }

        self.header
            .as_widget_mut()
            .operate(&mut tree.children[0], header, renderer, operation);
    }

    pub(super) fn operate_panel(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if is_open(tree) {
            self.panel.as_widget_mut().operate(
                &mut tree.children[1],
                parts(layout).1,
                renderer,
                operation,
            );
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Disclosure<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.header), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.header);
        tree.children[1].diff(&self.panel);

        if !self.enabled {
            set_open(tree, false);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let header = self.layout_header(tree, renderer, &limits);
        let size = header.size();
        let panel = is_open(tree).then(|| {
            self.layout_panel(
                tree,
                renderer,
                size.width,
                (limits.max().height - size.height - self.gap).max(0.0),
            )
            .move_to(Point::new(0.0, size.height + self.gap))
        });

        place(header, panel)
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
            self.operate_header(tree, layout, renderer, operation);
            self.operate_panel(tree, layout, renderer, operation);
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
        let (header, panel) = parts(layout);
        let state = tree.state.downcast_mut::<State>();

        if state.open {
            let mut messages = Vec::new();
            let mut panel_shell = Shell::new(&mut messages);
            self.panel.as_widget_mut().update(
                &mut tree.children[1],
                event,
                panel,
                cursor,
                renderer,
                clipboard,
                &mut panel_shell,
                viewport,
            );

            if popup::forward(shell, panel_shell) {
                state.open = false;
                shell.invalidate_layout();
            }
        }

        self.header.as_widget_mut().update(
            &mut tree.children[0],
            event,
            header,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if state.interaction.update(
            event,
            header.bounds().intersection(viewport).unwrap_or_default(),
            cursor,
            self.enabled,
            shell,
        ) || (self.dismissible && state.open && dismisses(event, cursor, layout.bounds()))
        {
            state.open = !state.open;
            shell.invalidate_layout();
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
        let (header, panel) = parts(layout);
        let mut interaction = self.header.as_widget().mouse_interaction(
            &tree.children[0],
            header,
            cursor,
            viewport,
            renderer,
        );

        if is_open(tree) {
            interaction = interaction.max(self.panel.as_widget().mouse_interaction(
                &tree.children[1],
                panel,
                cursor,
                viewport,
                renderer,
            ));
        }

        if interaction == mouse::Interaction::default()
            && self.enabled
            && cursor.is_over(header.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            interaction
        }
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
        let state = tree.state.downcast_ref::<State>();
        let (header, panel) = parts(layout);
        let colors = crate::theme::colors(theme);

        if state.open {
            draw_bridge(
                renderer,
                header.bounds(),
                panel.bounds(),
                colors.border,
                colors.background,
            );
        }

        draw_header(
            renderer,
            theme,
            header.bounds(),
            state
                .interaction
                .state(self.enabled, state.open, header.bounds(), cursor),
            false,
        );
        self.header.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: if self.enabled {
                    colors.text
                } else {
                    colors.muted
                },
            },
            header,
            cursor,
            viewport,
        );

        if state.open {
            self.panel.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                panel,
                cursor,
                viewport,
            );
        }

        draw_caret(renderer, theme, header.bounds(), state.open);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let open = is_open(tree);
        let (header, panel) = parts(layout);
        let (header_tree, panel_tree) = tree.children.split_at_mut(1);
        let mut overlays = Vec::new();

        overlays.extend(self.header.as_widget_mut().overlay(
            &mut header_tree[0],
            header,
            renderer,
            viewport,
            translation,
        ));

        if open {
            overlays.extend(
                self.panel
                    .as_widget_mut()
                    .overlay(&mut panel_tree[0], panel, renderer, viewport, translation)
                    .map(|overlay| overlay.map(&PopupMessage::into_inner)),
            );
        }

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: Clone + 'a> From<Disclosure<'a, Message>> for Element<'a, Message> {
    fn from(disclosure: Disclosure<'a, Message>) -> Self {
        Element::new(disclosure)
    }
}

/// Whether the disclosure whose tree is `tree` is open.
pub(super) fn is_open(tree: &Tree) -> bool {
    tree.state.downcast_ref::<State>().open
}

pub(super) fn set_open(tree: &mut Tree, open: bool) {
    tree.state.downcast_mut::<State>().open = open;
}

/// Joins a header and an open panel laid out where they go in the
/// container into the disclosure's node.
pub(super) fn place(header: layout::Node, panel: Option<layout::Node>) -> layout::Node {
    let bounds = panel.as_ref().map_or(header.bounds(), |panel| {
        header.bounds().union(&panel.bounds())
    });
    let offset = Point::ORIGIN - bounds.position();
    let panel = panel.map_or_else(
        || layout::Node::new(Size::ZERO),
        |panel| panel.translate(offset),
    );

    layout::Node::with_children(bounds.size(), vec![header.translate(offset), panel])
        .move_to(bounds.position())
}

/// Fills a disclosure's header as a list row that is `selected` while its
/// panel shows. An `attached` panel continues the header, so the header's
/// bottom corners go.
pub(super) fn draw_header(
    renderer: &mut iced::Renderer,
    theme: &Theme,
    bounds: Rectangle,
    state: ActionState,
    attached: bool,
) {
    let mut style = list_row::action_style(theme, state);
    if attached {
        style.border = Border::default().rounded(iced::border::top(RADIUS));
    }
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: style.border,
            shadow: style.shadow,
            snap: style.snap,
        },
        style
            .background
            .unwrap_or(Background::Color(Color::TRANSPARENT)),
    );
}

/// Draws the caret in the slot a disclosure's `header` keeps for it,
/// pointing up while `open`.
pub(super) fn draw_caret(
    renderer: &mut iced::Renderer,
    theme: &Theme,
    header: Rectangle,
    open: bool,
) {
    let slot = Rectangle {
        x: header.x + header.width - list_row::STANDARD_PADDING.right - CARET,
        y: header.center_y() - CARET / 2.0,
        width: CARET,
        height: CARET,
    };
    let handle = crate::icons::Icon::DownCaret.handle();
    let Size { width, height } = renderer.measure_svg(&handle);
    let size = ContentFit::Contain.fit(Size::new(width as f32, height as f32), slot.size());
    let bounds = Rectangle::new(
        Point::new(
            slot.center_x() - size.width / 2.0,
            slot.center_y() - size.height / 2.0,
        ),
        size,
    );

    renderer.draw_svg(
        iced::advanced::svg::Svg {
            handle,
            color: Some(crate::theme::colors(theme).muted),
            rotation: (std::f32::consts::PI * f32::from(open)).into(),
            opacity: 1.0,
        },
        bounds,
        slot,
    );
}

/// Joins an open panel to its header in the panel's colour, through the gap
/// between them and around the rounded corners they face each other with.
fn draw_bridge(
    renderer: &mut iced::Renderer,
    header: Rectangle,
    panel: Rectangle,
    color: Color,
    background: Color,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: header.x,
                y: header.y + header.height - RADIUS,
                width: header.width,
                height: panel.y - header.y - header.height + RADIUS * 2.0,
            },
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Background::Color(color),
    );

    // A panel wider than its header curves out into it.
    if header.x > panel.x {
        fill_concave_corner(renderer, header.x, panel.y, true, color, background);
    }

    if header.x + header.width < panel.x + panel.width {
        fill_concave_corner(
            renderer,
            header.x + header.width,
            panel.y,
            false,
            color,
            background,
        );
    }
}

fn fill_concave_corner(
    renderer: &mut iced::Renderer,
    x: f32,
    panel_y: f32,
    extends_left: bool,
    color: Color,
    background: Color,
) {
    let clip_x = if extends_left { x - RADIUS } else { x };
    let clip = Rectangle::new(
        Point::new(clip_x, panel_y - RADIUS),
        Size::new(RADIUS, RADIUS),
    );
    let cutout_x = if extends_left { x - RADIUS * 2.0 } else { x };
    let cutout = Rectangle::new(
        Point::new(cutout_x, panel_y - RADIUS * 2.0),
        Size::new(RADIUS * 2.0, RADIUS * 2.0),
    );

    renderer.with_layer(clip, |renderer| {
        renderer.fill_quad(
            renderer::Quad {
                bounds: clip,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(color),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: cutout,
                border: Border::default().rounded(RADIUS),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(background),
        );
    });
}

fn parts(layout: Layout<'_>) -> (Layout<'_>, Layout<'_>) {
    let mut children = layout.children();
    (
        children.next().expect("disclosure header"),
        children.next().expect("disclosure panel"),
    )
}

fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor
            .position()
            .is_some_and(|position| !bounds.contains(position)),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            repeat: false,
            ..
        }) => true,
        _ => false,
    }
}
