use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{column, container, text},
};

use super::{
    control::{
        ActivationSignal, Control, Interaction, State as ControlState, Style as ControlStyle,
    },
    draw_caret,
    expander_row::{ExpanderRow, header_row},
    list_row::{Content as RowContent, ListRow},
    spacing,
    surface::{Kind as SurfaceKind, Surface},
    text::TextExt as _,
};

const RADIUS: f32 = 6.0;

pub struct RowGroup<'a, Message> {
    title: Option<&'a str>,
    description: Option<&'a str>,
    columns: usize,
    enabled: bool,
    entries: Vec<Entry<'a, Message>>,
}

enum Entry<'a, Message> {
    Row(ListRow<'a, Message>),
    Expander(ExpanderRow<'a, Message>),
}

impl<'a, Message: 'a> RowGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            columns: 1,
            enabled: true,
            entries: Vec::new(),
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn row(mut self, row: impl Into<ListRow<'a, Message>>) -> Self {
        self.entries.push(Entry::Row(row.into()));
        self
    }

    pub fn expander(mut self, expander: ExpanderRow<'a, Message>) -> Self {
        self.entries.push(Entry::Expander(expander));
        self
    }
}

impl<'a, Message: 'a> Default for RowGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<RowGroup<'a, Message>> for Element<'a, Message> {
    fn from(group: RowGroup<'a, Message>) -> Self {
        let mut rows = column![].spacing(spacing::SM);
        let mut entries = group.entries.into_iter();

        loop {
            let line: Vec<_> = entries.by_ref().take(group.columns).collect();

            if line.is_empty() {
                break;
            }

            rows = rows.push(group_line(line, group.columns, group.enabled, false));
        }

        let mut content = column![].spacing(spacing::SM);

        if group.title.is_some() || group.description.is_some() {
            let mut heading = column![].spacing(spacing::XS);

            if let Some(title) = group.title {
                heading = heading.push(text(title).subtitle().medium());
            }

            if let Some(description) = group.description {
                heading = heading.push(text(description).detail().muted());
            }

            content = content.push(heading);
        }

        content.push(rows).into()
    }
}

fn group_line<'a, Message: Clone + 'a>(
    entries: Vec<Entry<'a, Message>>,
    columns: usize,
    enabled: bool,
    standalone: bool,
) -> Element<'a, Message> {
    let columns = columns.max(1);
    let mut headers = Vec::with_capacity(entries.len());
    let mut bodies = Vec::new();
    let mut expansions = Vec::new();

    for (header_index, entry) in entries.into_iter().enumerate() {
        match entry {
            Entry::Row(row) => {
                headers.push(row.into_control(enabled).into());
            }
            Entry::Expander(expander) => {
                let ExpanderRow {
                    header,
                    columns: requested_columns,
                    content,
                    content_enabled,
                    enabled: expander_enabled,
                } = expander;

                if content.is_empty() {
                    headers.push(
                        header_row(header, expander_enabled)
                            .into_control(enabled)
                            .into(),
                    );
                    continue;
                }

                let RowContent {
                    element: header,
                    selected,
                    focus_first,
                    disclosure_index,
                    ..
                } = header_row(header, expander_enabled).into_disclosure_content();
                let activation = ActivationSignal::default();
                let mut header = Control::new(header)
                    .width(Length::Fill)
                    .sensitive(enabled && expander_enabled)
                    .activation_signal(activation.clone());

                if focus_first {
                    header = header.focus_first_descendant();
                }
                let content_columns = if standalone {
                    requested_columns
                } else {
                    requested_columns.min(columns)
                };
                let body: Element<'a, Message> = Surface::new(
                    SurfaceKind::Panel,
                    container(
                        content
                            .into_iter()
                            .fold(RowGroup::new().columns(content_columns), RowGroup::row),
                    )
                    .width(Length::Fill)
                    .padding(spacing::MD),
                )
                .into();
                let body = Control::new(body)
                    .width(Length::Fill)
                    .sensitive(enabled && expander_enabled && content_enabled)
                    .style(disabled_subtree_style)
                    .into();

                headers.push(header.into());
                expansions.push(Expansion {
                    header_index,
                    content_index: bodies.len(),
                    content_columns,
                    footprint: footprint(
                        header_index,
                        if standalone {
                            1
                        } else {
                            requested_columns.min(columns)
                        },
                        columns,
                    ),
                    sensitive: enabled && expander_enabled,
                    selected,
                    disclosure_index: disclosure_index.expect("expander header has a caret slot"),
                    activation,
                });
                bodies.push(body);
            }
        }
    }

    let header_count = headers.len();

    for expansion in &mut expansions {
        expansion.content_index += header_count;
    }

    headers.extend(bodies);

    Element::new(GroupLine {
        children: headers,
        header_count,
        expansions,
        columns,
    })
}

pub(crate) fn standalone_expander<'a, Message: Clone + 'a>(
    expander: ExpanderRow<'a, Message>,
) -> Element<'a, Message> {
    group_line(vec![Entry::Expander(expander)], 1, true, true)
}

fn expander_header_style(
    theme: &Theme,
    interaction: &Interaction,
    sensitive: bool,
    selected: bool,
    expanded: bool,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> ControlStyle {
    let mut state = interaction.state(sensitive, true, bounds, cursor);
    state.selected = selected;
    state.expanded = expanded;
    super::list_row::style(theme, state)
}

fn disabled_subtree_style(theme: &Theme, state: ControlState) -> ControlStyle {
    ControlStyle {
        text_color: theme.palette().text,
        border: Border::default().rounded(RADIUS),
        foreground: (!state.sensitive).then_some(Background::Color(crate::theme::scrim(theme))),
        ..ControlStyle::default()
    }
}

struct Expansion {
    header_index: usize,
    content_index: usize,
    content_columns: usize,
    footprint: Footprint,
    sensitive: bool,
    selected: bool,
    disclosure_index: usize,
    activation: ActivationSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footprint {
    start: usize,
    end: usize,
}

impl Footprint {
    fn span(self) -> usize {
        self.end - self.start
    }

    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

struct GroupLine<'a, Message> {
    children: Vec<Element<'a, Message>>,
    header_count: usize,
    expansions: Vec<Expansion>,
    columns: usize,
}

#[derive(Default)]
struct State {
    open: Vec<usize>,
    signature: Vec<(usize, usize, usize, usize, usize)>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for GroupLine<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            signature: self.signature(),
            ..State::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let signature = self.signature();
        let state = tree.state.downcast_mut::<State>();

        if state.signature != signature {
            state.open.clear();
            state.signature = signature;
        }

        state.open.retain(|index| *index < self.expansions.len());
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
        let state = tree.state.downcast_ref::<State>();
        let width = limits.max().width;
        let cell_width = cell_width(width, self.columns);
        let loose_header_limits = layout::Limits::new(
            Size::new(cell_width, 0.0),
            Size::new(cell_width, limits.max().height),
        );
        let header_height = self.children[..self.header_count]
            .iter_mut()
            .zip(&mut tree.children[..self.header_count])
            .map(|(header, tree)| {
                header
                    .as_widget_mut()
                    .layout(tree, renderer, &loose_header_limits)
                    .size()
                    .height
            })
            .fold(0.0, f32::max);
        let header_limits = layout::Limits::new(
            Size::new(cell_width, header_height),
            Size::new(cell_width, header_height),
        );
        let mut children = Vec::with_capacity(self.children.len());

        for (index, (header, tree)) in self.children[..self.header_count]
            .iter_mut()
            .zip(&mut tree.children[..self.header_count])
            .enumerate()
        {
            children.push(
                header
                    .as_widget_mut()
                    .layout(tree, renderer, &header_limits)
                    .move_to(Point::new(index as f32 * (cell_width + spacing::MD), 0.0)),
            );
        }

        let body_top = header_height + spacing::MD;
        let mut body_height: f32 = 0.0;

        for (index, expansion) in self.expansions.iter().enumerate() {
            let content = &mut self.children[expansion.content_index];
            let tree = &mut tree.children[expansion.content_index];

            if state.open.contains(&index) {
                let panel_width = footprint_width(expansion.footprint, cell_width);
                let content_limits = layout::Limits::new(
                    Size::new(panel_width, 0.0),
                    Size::new(panel_width, limits.max().height),
                );
                let node = content
                    .as_widget_mut()
                    .layout(tree, renderer, &content_limits)
                    .move_to(Point::new(
                        expansion.footprint.start as f32 * (cell_width + spacing::MD),
                        body_top,
                    ));
                body_height = body_height.max(node.size().height);
                children.push(node);
            } else {
                children.push(layout::Node::new(Size::ZERO));
            }
        }

        let height = if state.open.is_empty() {
            header_height
        } else {
            body_top + body_height
        };

        layout::Node::with_children(Size::new(width, height), children)
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
            for ((child, child_tree), child_layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                if is_visible(child_layout) {
                    child
                        .as_widget_mut()
                        .operate(child_tree, child_layout, renderer, operation);
                }
            }
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
        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            if is_visible(child_layout) {
                child.as_widget_mut().update(
                    child_tree,
                    event,
                    child_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }

        let state = tree.state.downcast_mut::<State>();
        if self.apply_activations(&mut state.open) {
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
        let mut result = mouse::Interaction::default();

        for ((child, child_tree), child_layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            if is_visible(child_layout) {
                result = result.max(child.as_widget().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                ));
            }
        }

        result
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let children: Vec<_> = layout.children().collect();
        let palette = theme.extended_palette();
        let color = palette.background.neutral.color;
        let background = palette.background.base.color;

        for index in &state.open {
            let expansion = &self.expansions[*index];
            let header = children[expansion.header_index].bounds();
            let panel = children[expansion.content_index].bounds();

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: header.x,
                        y: header.y + header.height - RADIUS,
                        width: header.width,
                        height: panel.y - (header.y + header.height) + RADIUS * 2.0,
                    },
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(color),
            );

            if header.x > panel.x {
                fill_concave_corner(renderer, header.x, panel.y, true, color, background);
            }

            let header_right = header.x + header.width;
            let panel_right = panel.x + panel.width;

            if header_right < panel_right {
                fill_concave_corner(renderer, header_right, panel.y, false, color, background);
            }
        }

        for (index, expansion) in self.expansions.iter().enumerate() {
            let bounds = children[expansion.header_index].bounds();
            let interaction = tree.children[expansion.header_index]
                .state
                .downcast_ref::<Interaction>();
            let style = expander_header_style(
                theme,
                interaction,
                expansion.sensitive,
                expansion.selected,
                state.open.contains(&index),
                bounds,
                cursor,
            );

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                    snap: style.snap,
                },
                style
                    .background
                    .unwrap_or(Background::Color(iced::Color::TRANSPARENT)),
            );
        }

        for (index, child_layout) in children.iter().copied().enumerate() {
            if is_visible(child_layout) {
                self.children[index].as_widget().draw(
                    &tree.children[index],
                    renderer,
                    theme,
                    renderer_style,
                    child_layout,
                    cursor,
                    viewport,
                );
            }
        }

        for (index, expansion) in self.expansions.iter().enumerate() {
            draw_caret(
                renderer,
                theme,
                disclosure_bounds(children[expansion.header_index], expansion.disclosure_index),
                if state.open.contains(&index) {
                    1.0
                } else {
                    0.0
                },
            );

            if !expansion.sensitive {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: children[expansion.header_index].bounds(),
                        border: Border::default().rounded(RADIUS),
                        shadow: Shadow::default(),
                        snap: true,
                    },
                    Background::Color(crate::theme::scrim(theme)),
                );
            }
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let overlays = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((child, state), child_layout)| {
                if !is_visible(child_layout) {
                    return None;
                }

                child
                    .as_widget_mut()
                    .overlay(state, child_layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<Message> GroupLine<'_, Message> {
    fn signature(&self) -> Vec<(usize, usize, usize, usize, usize)> {
        self.expansions
            .iter()
            .map(|expansion| {
                (
                    self.columns,
                    expansion.header_index,
                    expansion.content_columns,
                    expansion.footprint.start,
                    expansion.footprint.end,
                )
            })
            .collect()
    }

    fn apply_activations(&self, open: &mut Vec<usize>) -> bool {
        let footprints: Vec<_> = self
            .expansions
            .iter()
            .map(|expansion| expansion.footprint)
            .collect();
        let mut changed = false;

        for (target, expansion) in self.expansions.iter().enumerate() {
            if expansion.activation.replace(false) {
                toggle_open(open, target, &footprints);
                changed = true;
            }
        }

        changed
    }
}

fn is_visible(layout: Layout<'_>) -> bool {
    let bounds = layout.bounds();
    bounds.width > 0.0 && bounds.height > 0.0
}

fn disclosure_bounds(header: Layout<'_>, index: usize) -> Rectangle {
    header
        .children()
        .next()
        .expect("expander control content")
        .children()
        .next()
        .expect("list row content")
        .children()
        .nth(index)
        .expect("expander disclosure slot")
        .bounds()
}

fn cell_width(width: f32, columns: usize) -> f32 {
    (width - spacing::MD * (columns.saturating_sub(1) as f32)).max(0.0) / columns as f32
}

fn fill_concave_corner(
    renderer: &mut iced::Renderer,
    x: f32,
    panel_y: f32,
    extends_left: bool,
    color: iced::Color,
    background: iced::Color,
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

fn footprint(header: usize, requested_span: usize, columns: usize) -> Footprint {
    let columns = columns.max(1);
    let span = requested_span.max(1).min(columns);
    let start = header.min(columns - span);

    Footprint {
        start,
        end: start + span,
    }
}

fn footprint_width(footprint: Footprint, cell_width: f32) -> f32 {
    let span = footprint.span();
    span as f32 * cell_width + span.saturating_sub(1) as f32 * spacing::MD
}

fn toggle_open(open: &mut Vec<usize>, target: usize, footprints: &[Footprint]) {
    if let Some(position) = open.iter().position(|index| *index == target) {
        open.remove(position);
        return;
    }

    let target_footprint = footprints[target];
    open.retain(|index| !footprints[*index].overlaps(target_footprint));
    open.push(target);
    open.sort_unstable();
}

#[cfg(test)]
mod tests {
    use super::{Footprint, RowGroup, toggle_open};

    #[test]
    fn opening_evicts_only_overlapping_expansions() {
        let footprints = [
            Footprint { start: 0, end: 1 },
            Footprint { start: 1, end: 3 },
            Footprint { start: 0, end: 2 },
        ];
        let mut open = Vec::new();

        toggle_open(&mut open, 0, &footprints);
        toggle_open(&mut open, 1, &footprints);
        assert_eq!(open, [0, 1]);

        toggle_open(&mut open, 2, &footprints);
        assert_eq!(open, [2]);

        toggle_open(&mut open, 2, &footprints);
        assert!(open.is_empty());
    }

    #[test]
    fn column_count_is_never_zero() {
        let group = RowGroup::<'static, ()>::new().columns(0);

        assert_eq!(group.columns, 1);
    }
}
