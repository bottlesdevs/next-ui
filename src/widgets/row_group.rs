use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{column, container, text},
};

use super::{
    expander_row::{ExpanderParts, ExpanderRow, control, passive},
    list_row::ListRow,
    pressable::SharedFlag,
    spacing,
    text::TextExt as _,
};

const RADIUS: f32 = 8.0;

pub struct RowGroup<'a, Message> {
    title: Option<&'a str>,
    description: Option<&'a str>,
    columns: usize,
    enabled: bool,
    entries: Vec<RowGroupEntry<'a, Message>>,
}

enum Entry<'a, Message> {
    Row(ListRow<'a, Message>),
    Expander(ExpanderParts<'a, Message>),
}

#[doc(hidden)]
pub struct RowGroupEntry<'a, Message> {
    entry: Entry<'a, Message>,
}

impl<'a, Message, T> From<T> for RowGroupEntry<'a, Message>
where
    T: Into<ListRow<'a, Message>>,
{
    fn from(row: T) -> Self {
        Self {
            entry: Entry::Row(row.into()),
        }
    }
}

impl<'a, Message> From<ExpanderRow<'a, Message>> for RowGroupEntry<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        Self {
            entry: Entry::Expander(expander.into_parts()),
        }
    }
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

    pub fn add(mut self, entry: impl Into<RowGroupEntry<'a, Message>>) -> Self {
        self.entries.push(entry.into());
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
        let mut rows = column![].spacing(spacing::MD);
        let mut entries = group.entries.into_iter();

        loop {
            let line: Vec<_> = entries.by_ref().take(group.columns).collect();

            if line.is_empty() {
                break;
            }

            rows = rows.push(group_line(line, group.columns, group.enabled, false));
        }

        let mut content = column![].spacing(spacing::MD);

        if group.title.is_some() || group.description.is_some() {
            let mut heading = column![].spacing(spacing::XS);

            if let Some(title) = group.title {
                heading = heading.push(text(title).subtitle());
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
    entries: Vec<RowGroupEntry<'a, Message>>,
    columns: usize,
    enabled: bool,
    standalone: bool,
) -> Element<'a, Message> {
    let columns = columns.max(1);
    let mut headers = Vec::with_capacity(entries.len());
    let mut bodies = Vec::new();
    let mut expansions = Vec::new();

    for (header_index, entry) in entries.into_iter().enumerate() {
        match entry.entry {
            Entry::Row(row) => headers.push(Element::from(row.parent_enabled(enabled))),
            Entry::Expander(parts) => {
                let ExpanderParts {
                    header,
                    columns: requested_columns,
                    content,
                    content_enabled,
                    enabled: expander_enabled,
                } = parts;
                let expander_enabled = enabled && expander_enabled;

                if content.is_empty() {
                    headers.push(Element::from(passive(header, expander_enabled)));
                    continue;
                }

                let activated = SharedFlag::default();
                let expanded = SharedFlag::default();
                let header = control(
                    header,
                    expander_enabled,
                    activated.clone(),
                    expanded.clone(),
                );
                let content_columns = if standalone {
                    requested_columns
                } else {
                    requested_columns.min(columns)
                };
                let body: Element<'a, Message> = container(
                    content.into_iter().fold(
                        RowGroup::new()
                            .columns(content_columns)
                            .enabled(expander_enabled && content_enabled),
                        RowGroup::add,
                    ),
                )
                .width(Length::Fill)
                .padding(spacing::MD)
                .into();

                headers.push(Element::from(header));
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
                    activated,
                    expanded,
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
    entry: RowGroupEntry<'a, Message>,
) -> Element<'a, Message> {
    group_line(vec![entry], 1, true, true)
}

struct Expansion {
    header_index: usize,
    content_index: usize,
    content_columns: usize,
    footprint: Footprint,
    activated: SharedFlag,
    expanded: SharedFlag,
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
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let signature: Vec<_> = self
            .expansions
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
            .collect();
        let state = tree.state.downcast_mut::<State>();

        if state.signature != signature {
            state.open.clear();
            state.signature = signature;
        }

        state.open.retain(|index| *index < self.expansions.len());
        self.sync_controls(state);
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
        self.sync_controls(state);
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
        let state = tree.state.downcast_ref::<State>();
        let layouts: Vec<_> = layout.children().collect();
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for index in 0..self.header_count {
                self.children[index].as_widget_mut().operate(
                    &mut tree.children[index],
                    layouts[index],
                    renderer,
                    operation,
                );
            }

            for index in &state.open {
                let content_index = self.expansions[*index].content_index;
                self.children[content_index].as_widget_mut().operate(
                    &mut tree.children[content_index],
                    layouts[content_index],
                    renderer,
                    operation,
                );
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
        let layouts: Vec<_> = layout.children().collect();
        let open = tree.state.downcast_ref::<State>().open.clone();

        for index in 0..self.header_count {
            self.children[index].as_widget_mut().update(
                &mut tree.children[index],
                event,
                layouts[index],
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        for index in open {
            let content_index = self.expansions[index].content_index;
            self.children[content_index].as_widget_mut().update(
                &mut tree.children[content_index],
                event,
                layouts[content_index],
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        let state = tree.state.downcast_mut::<State>();
        let was_open = state.open.clone();
        let footprints: Vec<_> = self
            .expansions
            .iter()
            .map(|expansion| expansion.footprint)
            .collect();

        for (index, expansion) in self.expansions.iter().enumerate() {
            if expansion.activated.take() {
                toggle_open(&mut state.open, index, &footprints);
            }
        }

        if state.open != was_open {
            self.sync_controls(state);
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
        let state = tree.state.downcast_ref::<State>();

        self.visible_indices(state)
            .map(|index| {
                self.children[index].as_widget().mouse_interaction(
                    &tree.children[index],
                    layout
                        .children()
                        .nth(index)
                        .expect("group line child layout"),
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
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
        self.sync_controls(state);
        let children: Vec<_> = layout.children().collect();
        let line_bounds = layout.bounds();
        let palette = theme.extended_palette();
        let color = palette.background.neutral.color;
        let background = palette.background.base.color;

        for index in &state.open {
            let expansion = &self.expansions[*index];
            let header = children[expansion.header_index].bounds();
            let content = children[expansion.content_index].bounds();
            let panel = Rectangle {
                height: line_bounds.y + line_bounds.height - content.y,
                ..content
            };

            renderer.fill_quad(
                renderer::Quad {
                    bounds: panel,
                    border: Border::default().rounded(RADIUS),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(color),
            );

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

        for index in self.visible_indices(state) {
            self.children[index].as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                style,
                children[index],
                cursor,
                viewport,
            );
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
            .filter(|(_, layout)| layout.bounds().width > 0.0 && layout.bounds().height > 0.0)
            .filter_map(|((child, state), layout)| {
                child
                    .as_widget_mut()
                    .overlay(state, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<Message> GroupLine<'_, Message> {
    fn sync_controls(&self, state: &State) {
        for (index, expansion) in self.expansions.iter().enumerate() {
            expansion.expanded.set(state.open.contains(&index));
        }
    }

    fn visible_indices<'a>(&'a self, state: &'a State) -> impl Iterator<Item = usize> + 'a {
        (0..self.header_count).chain(
            state
                .open
                .iter()
                .map(|index| self.expansions[*index].content_index),
        )
    }
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
    use super::{Footprint, toggle_open};

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
}
