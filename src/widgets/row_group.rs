use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{Grid, button, column, container, text},
};

use super::{
    draw_caret,
    expander_row::{ExpanderRow, header_row},
    focusable_action::Interaction,
    spacing,
    text::TextExt as _,
};

const RADIUS: f32 = 6.0;

pub struct RowGroup<'a, Message> {
    title: Option<&'a str>,
    description: Option<&'a str>,
    columns: usize,
    entries: Vec<Entry<'a, Message>>,
}

enum Entry<'a, Message> {
    Row(Element<'a, Message>),
    Expander(ExpanderRow<'a, Message>),
}

impl<'a, Message: 'a> RowGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            columns: 1,
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

    pub fn row(mut self, row: impl Into<Element<'a, Message>>) -> Self {
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
        let rows = Element::new(RowGroupCore::new(group.entries, group.columns, false));

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

struct RowGroupCore<'a, Message> {
    children: Vec<Element<'a, Message>>,
    header_count: usize,
    expansions: Vec<Expansion>,
    structure: Vec<Option<usize>>,
    columns: usize,
    line_count: usize,
}

impl<'a, Message: Clone + 'a> RowGroupCore<'a, Message> {
    fn new(entries: Vec<Entry<'a, Message>>, columns: usize, standalone: bool) -> Self {
        let columns = columns.max(1);
        let mut headers = Vec::with_capacity(entries.len());
        let mut bodies = Vec::new();
        let mut expansions = Vec::new();
        let mut structure = Vec::with_capacity(entries.len());

        for (header_index, entry) in entries.into_iter().enumerate() {
            match entry {
                Entry::Row(row) => {
                    structure.push(None);
                    headers.push(row);
                }
                Entry::Expander(expander) => {
                    let ExpanderRow {
                        header,
                        columns: requested_columns,
                        content,
                    } = expander;

                    if content.is_empty() {
                        structure.push(None);
                        headers.push(header_row(header).into());
                        continue;
                    }
                    structure.push(Some(requested_columns));

                    let header = header_row(header).into_disclosure_content();
                    let content_columns = if standalone {
                        requested_columns
                    } else {
                        requested_columns.min(columns)
                    };
                    let body: Element<'a, Message> = container(
                        Grid::with_children(content)
                            .columns(content_columns)
                            .spacing(spacing::SM)
                            .height(Length::Shrink),
                    )
                    .width(Length::Fill)
                    .padding(spacing::MD)
                    .style(panel_style)
                    .into();

                    headers.push(header);
                    expansions.push(Expansion {
                        header_index,
                        content_index: bodies.len(),
                        footprint: footprint(
                            header_index,
                            if standalone { 1 } else { requested_columns },
                            columns,
                        ),
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

        Self {
            children: headers,
            header_count,
            expansions,
            structure,
            columns,
            line_count: header_count.div_ceil(columns),
        }
    }
}

pub(crate) fn standalone_expander<'a, Message: Clone + 'a>(
    expander: ExpanderRow<'a, Message>,
) -> Element<'a, Message> {
    Element::new(RowGroupCore::new(vec![Entry::Expander(expander)], 1, true))
}

fn expander_header_style(
    theme: &Theme,
    interaction: &Interaction,
    expanded: bool,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> button::Style {
    let mut state = interaction.state(true, false, bounds, cursor);
    state.selected = expanded;
    super::list_row::action_style(theme, state)
}

fn panel_style(theme: &Theme) -> container::Style {
    let colors = crate::theme::colors(theme);

    container::Style::default()
        .background(colors.border)
        .color(colors.text)
        .border(Border::default().rounded(RADIUS))
}

struct Expansion {
    header_index: usize,
    content_index: usize,
    footprint: Footprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Footprint {
    line: usize,
    start: usize,
    end: usize,
}

impl Footprint {
    fn span(self) -> usize {
        self.end - self.start
    }

    fn overlaps(self, other: Self) -> bool {
        self.line == other.line && self.start < other.end && other.start < self.end
    }
}

#[derive(Default)]
struct State {
    open: Vec<usize>,
    interactions: Vec<Interaction>,
    structure: Vec<Option<usize>>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for RowGroupCore<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            interactions: std::iter::repeat_with(Interaction::default)
                .take(self.expansions.len())
                .collect(),
            structure: self.structure.clone(),
            ..State::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_ref::<State>();
        let structure_changed = state.structure != self.structure;

        if structure_changed {
            tree.children = self.children();
        } else {
            tree.diff_children(&self.children);
        }

        let state = tree.state.downcast_mut::<State>();
        state.structure.clone_from(&self.structure);
        if structure_changed {
            state.open.clear();
            state.interactions = std::iter::repeat_with(Interaction::default)
                .take(self.expansions.len())
                .collect();
        } else {
            reconcile_open(&mut state.open, &self.expansions);
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
        let state = tree.state.downcast_ref::<State>();
        let width = limits.max().width;
        let cell_width = cell_width(width, self.columns);
        let loose_header_limits = layout::Limits::new(
            Size::new(cell_width, 0.0),
            Size::new(cell_width, limits.max().height),
        );
        let mut children: Vec<Option<layout::Node>> = std::iter::repeat_with(|| None)
            .take(self.children.len())
            .collect();
        let mut top = 0.0;

        for line in 0..self.line_count {
            let header_start = line * self.columns;
            let headers = header_start..(header_start + self.columns).min(self.header_count);
            let header_height = headers
                .clone()
                .map(|index| {
                    self.children[index]
                        .as_widget_mut()
                        .layout(&mut tree.children[index], renderer, &loose_header_limits)
                        .size()
                        .height
                })
                .fold(0.0, f32::max);
            let header_limits = layout::Limits::new(
                Size::new(cell_width, header_height),
                Size::new(cell_width, header_height),
            );

            for (column, index) in headers.enumerate() {
                children[index] = Some(
                    self.children[index]
                        .as_widget_mut()
                        .layout(&mut tree.children[index], renderer, &header_limits)
                        .move_to(Point::new(column as f32 * (cell_width + spacing::MD), top)),
                );
            }

            let body_top = top + header_height + spacing::MD;
            let mut body_height: f32 = 0.0;
            let mut line_open = false;

            for expansion in &self.expansions {
                if expansion.footprint.line != line || !state.open.contains(&expansion.header_index)
                {
                    continue;
                }

                line_open = true;
                let panel_width = footprint_width(expansion.footprint, cell_width);
                let content_limits = layout::Limits::new(
                    Size::new(panel_width, 0.0),
                    Size::new(panel_width, limits.max().height),
                );
                let node = self.children[expansion.content_index]
                    .as_widget_mut()
                    .layout(
                        &mut tree.children[expansion.content_index],
                        renderer,
                        &content_limits,
                    )
                    .move_to(Point::new(
                        expansion.footprint.start as f32 * (cell_width + spacing::MD),
                        body_top,
                    ));
                body_height = body_height.max(node.size().height);
                children[expansion.content_index] = Some(node);
            }

            top += if line_open {
                header_height + spacing::MD + body_height
            } else {
                header_height
            };

            if line + 1 < self.line_count {
                top += spacing::SM;
            }
        }

        layout::Node::with_children(
            Size::new(width, top),
            children
                .into_iter()
                .map(|child| child.unwrap_or_else(|| layout::Node::new(Size::ZERO)))
                .collect(),
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let children: Vec<_> = layout.children().collect();
        let order = visual_order(self.header_count, self.columns, &self.expansions);
        let state = tree.state.downcast_mut::<State>();

        for (interaction, expansion) in state.interactions.iter_mut().zip(&self.expansions) {
            operation.focusable(None, children[expansion.header_index].bounds(), interaction);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for index in &order {
                if is_visible(children[*index]) {
                    self.children[*index].as_widget_mut().operate(
                        &mut tree.children[*index],
                        children[*index],
                        renderer,
                        operation,
                    );
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

        let children: Vec<_> = layout.children().collect();
        let state = tree.state.downcast_mut::<State>();
        let activated = state
            .interactions
            .iter_mut()
            .zip(&self.expansions)
            .find_map(|(interaction, expansion)| {
                interaction
                    .update(
                        event,
                        children[expansion.header_index]
                            .bounds()
                            .intersection(viewport)
                            .unwrap_or(Rectangle {
                                x: children[expansion.header_index].bounds().x,
                                y: children[expansion.header_index].bounds().y,
                                width: 0.0,
                                height: 0.0,
                            }),
                        cursor,
                        true,
                        shell,
                    )
                    .then_some(expansion.header_index)
            });

        if let Some(header_index) = activated {
            toggle_open(&mut state.open, header_index, &self.expansions);
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

        if result == mouse::Interaction::default()
            && self.expansions.iter().any(|expansion| {
                cursor.is_over(
                    layout
                        .children()
                        .nth(expansion.header_index)
                        .expect("expander header layout")
                        .bounds(),
                )
            })
        {
            mouse::Interaction::Pointer
        } else {
            result
        }
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
        let colors = crate::theme::colors(theme);

        for header_index in &state.open {
            let expansion = self
                .expansions
                .iter()
                .find(|expansion| expansion.header_index == *header_index)
                .expect("open expander is present in the current group");
            let header = children[expansion.header_index].bounds();
            let panel = children[expansion.content_index].bounds();

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
                Background::Color(colors.border),
            );

            if header.x > panel.x {
                fill_concave_corner(
                    renderer,
                    header.x,
                    panel.y,
                    true,
                    colors.border,
                    colors.background,
                );
            }

            if header.x + header.width < panel.x + panel.width {
                fill_concave_corner(
                    renderer,
                    header.x + header.width,
                    panel.y,
                    false,
                    colors.border,
                    colors.background,
                );
            }
        }

        for (expansion, interaction) in self.expansions.iter().zip(&state.interactions) {
            let bounds = children[expansion.header_index].bounds();
            let style = expander_header_style(
                theme,
                interaction,
                state.open.contains(&expansion.header_index),
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

        for expansion in &self.expansions {
            draw_caret(
                renderer,
                theme,
                disclosure_bounds(children[expansion.header_index].bounds()),
                if state.open.contains(&expansion.header_index) {
                    1.0
                } else {
                    0.0
                },
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

fn is_visible(layout: Layout<'_>) -> bool {
    let bounds = layout.bounds();
    bounds.width > 0.0 && bounds.height > 0.0
}

fn disclosure_bounds(header: Rectangle) -> Rectangle {
    const SIZE: f32 = 20.0;

    Rectangle {
        x: header.x + header.width - super::list_row::STANDARD_PADDING.right - SIZE,
        y: header.center_y() - SIZE / 2.0,
        width: SIZE,
        height: SIZE,
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
    let line = header / columns;
    let column = header % columns;
    let start = column.min(columns - span);

    Footprint {
        line,
        start,
        end: start + span,
    }
}

fn footprint_width(footprint: Footprint, cell_width: f32) -> f32 {
    let span = footprint.span();
    span as f32 * cell_width + span.saturating_sub(1) as f32 * spacing::MD
}

fn visual_order(header_count: usize, columns: usize, expansions: &[Expansion]) -> Vec<usize> {
    let mut order = Vec::with_capacity(header_count + expansions.len());

    for line in 0..header_count.div_ceil(columns) {
        let header_start = line * columns;
        order.extend(header_start..(header_start + columns).min(header_count));
        order.extend(
            expansions
                .iter()
                .filter(|expansion| expansion.footprint.line == line)
                .map(|expansion| expansion.content_index),
        );
    }

    order
}

fn reconcile_open(open: &mut Vec<usize>, expansions: &[Expansion]) {
    let previous = std::mem::take(open);

    for expansion in expansions {
        if previous.contains(&expansion.header_index) {
            open.retain(|header_index| {
                expansions
                    .iter()
                    .find(|candidate| candidate.header_index == *header_index)
                    .is_some_and(|candidate| !candidate.footprint.overlaps(expansion.footprint))
            });
            open.push(expansion.header_index);
        }
    }
}

fn toggle_open(open: &mut Vec<usize>, target: usize, expansions: &[Expansion]) {
    if let Some(position) = open.iter().position(|index| *index == target) {
        open.remove(position);
        return;
    }

    let target_footprint = expansions
        .iter()
        .find(|expansion| expansion.header_index == target)
        .expect("activated expander is present in the current group")
        .footprint;
    open.retain(|header_index| {
        expansions
            .iter()
            .find(|expansion| expansion.header_index == *header_index)
            .is_some_and(|expansion| !expansion.footprint.overlaps(target_footprint))
    });
    open.push(target);
    open.sort_unstable();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expansions(columns: usize, specs: &[(usize, usize)]) -> Vec<Expansion> {
        let header_count = specs.iter().map(|(header, _)| *header).max().unwrap_or(0) + 1;

        specs
            .iter()
            .enumerate()
            .map(|(index, (header, span))| Expansion {
                header_index: *header,
                content_index: header_count + index,
                footprint: footprint(*header, *span, columns),
            })
            .collect()
    }

    #[test]
    fn toggles_keep_disjoint_panels_and_evict_overlaps() {
        let expansions = expansions(3, &[(0, 1), (1, 2), (2, 3)]);
        let mut open = Vec::new();

        toggle_open(&mut open, 0, &expansions);
        toggle_open(&mut open, 1, &expansions);
        assert_eq!(open, [0, 1]);

        toggle_open(&mut open, 2, &expansions);
        assert_eq!(open, [2]);

        toggle_open(&mut open, 2, &expansions);
        assert!(open.is_empty());
    }

    #[test]
    fn column_count_is_never_zero() {
        let group = RowGroup::<'static, ()>::new().columns(0);

        assert_eq!(group.columns, 1);
    }
}
