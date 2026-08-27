use iced::{
    Alignment, Background, Border, Element, Event, Fill, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    keyboard::{self, key},
    touch,
    widget::{Space, button, column, row, svg, text, text::Fragment},
};

use crate::icons::Icon;

use super::{
    control::{Control, State, focus_first_descendant},
    popover::State as PopoverState,
    spacing,
    surface::{Kind as SurfaceKind, scoped_overlay},
    text::TextExt as _,
};

#[derive(Clone, Copy)]
enum Placement {
    Search,
    Popover,
}

enum Owner<'a> {
    Search(&'a mut bool),
    Popover(&'a mut PopoverState),
}

impl Owner<'_> {
    fn placement(&self) -> Placement {
        match self {
            Self::Search(_) => Placement::Search,
            Self::Popover(_) => Placement::Popover,
        }
    }

    fn dismiss(&mut self) {
        match self {
            Self::Search(dismissed) => **dismissed = true,
            Self::Popover(state) => {
                state.open = false;
                state.focus_panel = false;
                state.focus_trigger = true;
            }
        }
    }
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

pub(super) fn row_content<'a, Message: 'a>(
    title: Fragment<'a>,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
) -> iced::widget::Row<'a, Message> {
    let mut labels = column![text(title).label()].spacing(spacing::XS);

    if let Some(subtitle) = subtitle {
        labels = labels.push(text(subtitle).detail().muted());
    }

    let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        content = content.push(
            svg(icon.handle())
                .width(20)
                .height(20)
                .content_fit(iced::ContentFit::Contain),
        );
    }

    content.push(labels).push(Space::new().width(Fill))
}

pub(super) fn footer<'a, Message: Clone + 'a>(
    label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    Control::new(
        row![text(label), Icon::Arrow.rotated(std::f32::consts::PI)]
            .spacing(spacing::SM)
            .align_y(Alignment::Center),
    )
    .padding(spacing::MD)
    .on_press(message)
    .style(footer_style)
    .into()
}

pub(super) fn row_style(theme: &Theme, state: State) -> button::Style {
    let highlighted = state.actionable
        && (state.keyboard_highlighted || state.hovered || state.pressed || state.focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}

fn footer_style(theme: &Theme, state: State) -> button::Style {
    let colors = if state.hovered || state.pressed {
        theme.extended_palette().background.strongest
    } else {
        theme.extended_palette().background.stronger
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: theme.extended_palette().secondary.weak.text,
        border: Border::default().rounded(iced::border::bottom(6)),
        ..button::Style::default()
    }
}

/// The common body/footer plumbing for panels anchored to another widget.
pub(super) struct AnchoredPanel<'a, 'b, Message>
where
    'b: 'a,
{
    position: Point,
    target_height: f32,
    width: f32,
    viewport: Rectangle,
    content: &'a mut PanelContent<'b, Message>,
    tree: &'a mut Tree,
    owner: Owner<'a>,
}

impl<'a, 'b, Message> AnchoredPanel<'a, 'b, Message>
where
    'b: 'a,
{
    pub(super) fn search(
        position: Point,
        target_height: f32,
        width: f32,
        viewport: Rectangle,
        content: &'a mut PanelContent<'b, Message>,
        tree: &'a mut Tree,
        dismissed: &'a mut bool,
    ) -> Self {
        Self {
            position,
            target_height,
            width,
            viewport,
            content,
            tree,
            owner: Owner::Search(dismissed),
        }
    }

    pub(super) fn popover(
        position: Point,
        target_height: f32,
        width: f32,
        viewport: Rectangle,
        content: &'a mut PanelContent<'b, Message>,
        tree: &'a mut Tree,
        state: &'a mut PopoverState,
    ) -> Self {
        Self {
            position,
            target_height,
            width,
            viewport,
            content,
            tree,
            owner: Owner::Popover(state),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    min_width: f32,
    max_width: f32,
    below: f32,
    above: f32,
    open_below: Option<bool>,
    inset: f32,
    viewport: Rectangle,
}

fn geometry(
    placement: Placement,
    position: Point,
    target_height: f32,
    width: f32,
    viewport: Rectangle,
) -> Geometry {
    let gap = spacing::XS;
    let inset = if matches!(placement, Placement::Popover) {
        spacing::SM
    } else {
        0.0
    };
    let max_width = (viewport.width - inset * 2.0).max(0.0);
    let min_width = width.min(max_width);
    let below = viewport.y + viewport.height - inset - (position.y + target_height + gap);
    let above = position.y - gap - (viewport.y + inset);

    Geometry {
        min_width,
        max_width: if matches!(placement, Placement::Search) {
            min_width
        } else {
            max_width
        },
        below,
        above,
        open_below: matches!(placement, Placement::Popover).then_some(below >= above),
        inset,
        viewport,
    }
}

fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => {
            !cursor.is_over(bounds) && cursor.position().is_some()
        }
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        }) => true,
        _ => false,
    }
}

fn body_height_limit(max_height: f32, footer_height: f32) -> f32 {
    (max_height - footer_height).max(0.0)
}

fn panel_width(geometry: Geometry, measured: f32) -> f32 {
    measured.clamp(geometry.min_width, geometry.max_width)
}

impl Geometry {
    fn position(self, anchor: Point, target_height: f32, width: f32, height: f32) -> Point {
        let gap = spacing::XS;
        let open_below = self
            .open_below
            .unwrap_or(self.below >= height || self.below >= self.above);
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
    for AnchoredPanel<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let bounds = Rectangle::with_size(bounds);
        let viewport = self.viewport.intersection(&bounds).unwrap_or(bounds);
        let placement = self.owner.placement();
        let geometry = geometry(
            placement,
            self.position,
            self.target_height,
            self.width,
            viewport,
        );
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

        let width = if matches!(placement, Placement::Popover) {
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
            Size::new(width, body_height_limit(max_height, footer_height)),
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
            self.position,
            self.target_height,
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
        if dismisses(event, cursor, layout.bounds()) {
            self.owner.dismiss();
            shell.capture_event();
            shell.request_redraw();

            return;
        }

        if let Owner::Popover(state) = &mut self.owner
            && state.focus_panel
        {
            if let (Some(body), Some(body_tree), Some(body_layout)) = (
                self.content.children.first_mut(),
                self.tree.children.first_mut(),
                layout.children().next(),
            ) {
                focus_first_descendant(body, body_tree, body_layout, renderer);
            }

            state.focus_panel = false;
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

        if !panel_shell.is_empty() {
            self.owner.dismiss();
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
    fn popover_width_uses_anchor_floor_and_viewport_cap() {
        let geometry = geometry(
            Placement::Popover,
            Point::new(20.0, 20.0),
            32.0,
            80.0,
            Rectangle::new(Point::new(10.0, 10.0), Size::new(300.0, 200.0)),
        );

        assert_eq!(panel_width(geometry, 40.0), 80.0);
        assert_eq!(panel_width(geometry, 140.0), 140.0);
        assert_eq!(panel_width(geometry, 500.0), 276.0);
    }

    #[test]
    fn search_position_is_clamped_to_the_viewport() {
        let anchor = Point::new(280.0, 50.0);
        let geometry = geometry(
            Placement::Search,
            anchor,
            30.0,
            100.0,
            Rectangle::new(Point::new(20.0, 10.0), Size::new(300.0, 200.0)),
        );

        assert_eq!(geometry.position(anchor, 30.0, 100.0, 50.0).x, 220.0);
    }

    #[test]
    fn touch_outside_panel_dismisses_it() {
        let event = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(0),
            position: Point::new(5.0, 5.0),
        });

        assert!(dismisses(
            &event,
            mouse::Cursor::Unavailable,
            Rectangle::new(Point::new(10.0, 10.0), Size::new(100.0, 100.0)),
        ));
    }
}
