use iced::{
    Alignment, Background, Border, Element, Event, Fill, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    keyboard::{self, key},
    widget::{Space, button, column, row, svg, text},
};

use crate::icons::Icon;

use super::{
    control::{Control, State},
    spacing,
    surface::{Kind as SurfaceKind, scoped_overlay},
    text::TextExt as _,
};

#[derive(Clone, Copy)]
enum Placement {
    Search,
    Popover,
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
    title: &'a str,
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
    .width(Fill)
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
    placement: Placement,
    on_dismiss: Option<Message>,
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
    ) -> Self {
        Self {
            position,
            target_height,
            width,
            viewport,
            content,
            tree,
            placement: Placement::Search,
            on_dismiss: None,
        }
    }

    pub(super) fn popover(
        position: Point,
        target_height: f32,
        width: f32,
        viewport: Rectangle,
        content: &'a mut PanelContent<'b, Message>,
        tree: &'a mut Tree,
        on_dismiss: Option<Message>,
    ) -> Self {
        Self {
            position,
            target_height,
            width: width.max(240.0),
            viewport,
            content,
            tree,
            placement: Placement::Popover,
            on_dismiss,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    width: f32,
    x: f32,
    below: f32,
    above: f32,
    open_below: Option<bool>,
    inset: f32,
}

fn geometry(
    placement: Placement,
    position: Point,
    target_height: f32,
    width: f32,
    bounds: Size,
) -> Geometry {
    let gap = spacing::XS;

    match placement {
        Placement::Search => {
            let below = bounds.height - (position.y + target_height + gap);
            let above = position.y - gap;

            Geometry {
                width,
                x: position.x,
                below,
                above,
                open_below: None,
                inset: 0.0,
            }
        }
        Placement::Popover => {
            let inset = spacing::SM;
            let width = width.min((bounds.width - inset * 2.0).max(0.0));
            let below = bounds.height - (position.y + target_height + gap) - inset;
            let above = position.y - gap - inset;

            Geometry {
                width,
                x: position
                    .x
                    .clamp(inset, (bounds.width - width - inset).max(inset)),
                below,
                above,
                open_below: Some(below >= above),
                inset,
            }
        }
    }
}

fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => {
            !cursor.is_over(bounds) && cursor.position().is_some()
        }
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

fn panel_height(
    placement: Placement,
    max_height: f32,
    body_height: f32,
    footer_height: f32,
) -> f32 {
    let height = body_height + footer_height;

    if matches!(placement, Placement::Popover) {
        height.min(max_height)
    } else {
        height
    }
}

impl Geometry {
    fn position(self, anchor: Point, target_height: f32, bounds: Size, height: f32) -> Point {
        let gap = spacing::XS;
        let open_below = self
            .open_below
            .unwrap_or(self.below >= height || self.below >= self.above);
        let y = if open_below {
            anchor.y + target_height + gap
        } else {
            anchor.y - height - gap
        };
        let y = if self.inset > 0.0 {
            y.clamp(
                self.inset,
                (bounds.height - height - self.inset).max(self.inset),
            )
        } else {
            y
        };

        Point::new(self.x, y)
    }
}

impl<Message: Clone> iced::advanced::Overlay<Message, Theme, iced::Renderer>
    for AnchoredPanel<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let geometry = geometry(
            self.placement,
            self.position,
            self.target_height,
            self.width,
            bounds,
        );
        let max_height = geometry.below.max(geometry.above).max(0.0);
        let limits = layout::Limits::new(
            Size::new(geometry.width, 0.0),
            Size::new(geometry.width, max_height),
        );
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
        let footer = footer
            .first_mut()
            .zip(footer_trees.first_mut())
            .map(|(footer, tree)| footer.as_widget_mut().layout(tree, renderer, &limits));
        let footer_height = footer.as_ref().map_or(0.0, |node| node.size().height);
        let body_limits = layout::Limits::new(
            Size::new(geometry.width, 0.0),
            Size::new(geometry.width, body_height_limit(max_height, footer_height)),
        );
        let body = body
            .as_widget_mut()
            .layout(body_tree, renderer, &body_limits);
        let body_height = body.size().height;
        let height = panel_height(self.placement, max_height, body_height, footer_height);
        let mut children = vec![body];

        if let Some(footer) = footer {
            children.push(footer.move_to(Point::new(0.0, body_height)));
        }

        layout::Node::with_children(Size::new(geometry.width, height), children)
            .move_to(geometry.position(self.position, self.target_height, bounds, height))
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
        if matches!(self.placement, Placement::Popover) && dismisses(event, cursor, layout.bounds())
        {
            if let Some(message) = self.on_dismiss.clone() {
                shell.publish(message);
                shell.capture_event();
            }

            return;
        }

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
