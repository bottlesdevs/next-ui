use iced::{
    Alignment, Border, Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    border::Radius,
    widget::{
        Column, Grid, column, container, row, text,
        text::{Fragment, IntoFragment},
    },
};

use crate::icons::Icon;

use super::{
    disclosure::{self, Disclosure},
    expander_row::ExpanderRow,
    list_row::ListRow,
    popup::PopupMessage,
    radius, spacing,
    text::TextExt as _,
};

/// The hairline between a boxed group's rows.
const BOXED_GAP: f32 = 2.0;

/// The toolkit's one group of rows, in either of two styles: separate cards
/// in a grid under a heading, or one joined block (see [`BoxedGroup`]).
///
/// [`BoxedGroup`]: super::boxed_group::BoxedGroup
pub struct RowGroup<'a, Message> {
    title: Option<Fragment<'a>>,
    description: Option<Fragment<'a>>,
    columns: usize,
    style: Style,
    entries: Vec<Entry<'a, Message>>,
}

enum Style {
    Separate,
    /// Rows joined into one block under an optional heading row: only the
    /// block's outer corners are rounded, and the rows sit a hairline apart.
    Boxed {
        icon: Option<Icon>,
    },
}

enum Entry<'a, Message> {
    Element(Element<'a, Message>),
    /// A row kept unlowered so a boxed group can give it the corners its
    /// place there needs.
    Row(ListRow<'a, Message>),
    Expander(ExpanderRow<'a, Message>),
}

impl<'a, Message: 'a> RowGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            columns: 1,
            style: Style::Separate,
            entries: Vec::new(),
        }
    }

    pub(crate) fn boxed() -> Self {
        Self {
            style: Style::Boxed { icon: None },
            ..Self::new()
        }
    }

    pub fn title(mut self, title: impl IntoFragment<'a>) -> Self {
        self.title = Some(title.into_fragment());
        self
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Shows `icon` before a boxed group's title.
    pub(crate) fn icon(mut self, icon: Icon) -> Self {
        if let Style::Boxed { icon: current } = &mut self.style {
            *current = Some(icon);
        }
        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn row(mut self, row: impl Into<Element<'a, Message>>) -> Self {
        self.entries.push(Entry::Element(row.into()));
        self
    }

    pub(crate) fn list_row(mut self, row: impl Into<ListRow<'a, Message>>) -> Self {
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
        match group.style {
            Style::Separate => separate(group),
            Style::Boxed { icon } => boxed(group.title, icon, group.entries),
        }
    }
}

fn separate<'a, Message: Clone + 'a>(group: RowGroup<'a, Message>) -> Element<'a, Message> {
    let rows = Element::new(RowGroupCore::new(group.entries, group.columns));

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

fn boxed<'a, Message: Clone + 'a>(
    title: Option<Fragment<'a>>,
    icon: Option<Icon>,
    entries: Vec<Entry<'a, Message>>,
) -> Element<'a, Message> {
    let count = entries.len() + usize::from(title.is_some());
    let corners = |index: usize| Radius {
        top_left: if index == 0 { radius::BOX } else { 0.0 },
        top_right: if index == 0 { radius::BOX } else { 0.0 },
        bottom_left: if index + 1 == count { radius::BOX } else { 0.0 },
        bottom_right: if index + 1 == count { radius::BOX } else { 0.0 },
    };
    let mut block = Column::new().spacing(BOXED_GAP).width(Length::Fill);
    let mut index = 0;
    if let Some(title) = title {
        let mut heading = row![].spacing(spacing::SM).align_y(Alignment::Center);
        if let Some(icon) = icon {
            heading = heading.push(icon.view().width(18).height(18));
        }
        heading = heading.push(text(title).size(15).medium());
        let radius = corners(index);
        block = block.push(
            container(heading)
                .width(Length::Fill)
                .padding([spacing::SM, spacing::LG])
                .style(move |theme| {
                    let colors = crate::theme::colors(theme);
                    container::Style::default()
                        .background(colors.card)
                        .color(colors.accent_muted)
                        .border(Border::default().rounded(radius))
                }),
        );
        index += 1;
    }
    for entry in entries {
        block = block.push(match entry {
            Entry::Row(row) => row.grouped(corners(index)).into(),
            Entry::Element(element) => element,
            Entry::Expander(expander) => standalone_expander(expander),
        });
        index += 1;
    }
    block.into()
}

/// The separate style's grid. An expander's panel opens under its whole line
/// of the grid, spanning as many cells as its rows ask for, and closes any
/// open panel it would overlap.
struct RowGroupCore<'a, Message> {
    cells: Vec<Cell<'a, Message>>,
    columns: usize,
}

enum Cell<'a, Message> {
    Row(Element<'a, Message>),
    Expander(Disclosure<'a, Message>, Footprint),
}

impl<'a, Message: Clone + 'a> RowGroupCore<'a, Message> {
    fn new(entries: Vec<Entry<'a, Message>>, columns: usize) -> Self {
        let columns = columns.max(1);
        let cells = entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| match entry {
                Entry::Element(row) => Cell::Row(row),
                Entry::Row(row) => Cell::Row(row.into()),
                Entry::Expander(expander) if expander.content.is_empty() => {
                    Cell::Row(expander.header.into())
                }
                Entry::Expander(expander) => {
                    let footprint = footprint(index, expander.columns, columns);
                    Cell::Expander(disclosure(expander, columns), footprint)
                }
            })
            .collect();

        Self { cells, columns }
    }
}

impl<'a, Message: Clone> Cell<'a, Message> {
    fn widget(&self) -> &dyn Widget<Message, Theme, iced::Renderer> {
        match self {
            Self::Row(row) => row.as_widget(),
            Self::Expander(disclosure, _) => disclosure,
        }
    }

    fn widget_mut(&mut self) -> &mut dyn Widget<Message, Theme, iced::Renderer> {
        match self {
            Self::Row(row) => row.as_widget_mut(),
            Self::Expander(disclosure, _) => disclosure,
        }
    }
}

/// An expander's header over its rows, laid out `columns` wide at most.
fn disclosure<'a, Message: Clone + 'a>(
    expander: ExpanderRow<'a, Message>,
    columns: usize,
) -> Disclosure<'a, Message> {
    let panel: Element<'a, Message> = container(
        Grid::with_children(expander.content)
            .columns(expander.columns.min(columns))
            .spacing(spacing::SM)
            .height(Length::Shrink),
    )
    .width(Length::Fill)
    .padding(spacing::MD)
    .style(panel_style)
    .into();

    Disclosure::new(
        expander.header.into_disclosure_content(),
        panel.map(PopupMessage::Emit),
    )
    .gap(spacing::SM)
}

pub(crate) fn standalone_expander<'a, Message: Clone + 'a>(
    expander: ExpanderRow<'a, Message>,
) -> Element<'a, Message> {
    if expander.content.is_empty() {
        return expander.header.into();
    }
    let columns = expander.columns;
    disclosure(expander, columns).into()
}

fn panel_style(theme: &Theme) -> container::Style {
    let colors = crate::theme::colors(theme);

    container::Style::default()
        .background(colors.raised)
        .color(colors.text)
        .border(Border::default().rounded(radius::ROW))
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

impl<'a, Message: Clone> RowGroupCore<'a, Message> {
    fn footprints(&self) -> Vec<Option<Footprint>> {
        self.cells
            .iter()
            .map(|cell| match cell {
                Cell::Row(_) => None,
                Cell::Expander(_, footprint) => Some(*footprint),
            })
            .collect()
    }

    fn open(&self, tree: &Tree) -> Vec<bool> {
        self.cells
            .iter()
            .zip(&tree.children)
            .map(|(cell, tree)| matches!(cell, Cell::Expander(..)) && disclosure::is_open(tree))
            .collect()
    }

    fn set_open(&self, tree: &mut Tree, open: &[bool]) {
        for ((cell, tree), open) in self.cells.iter().zip(&mut tree.children).zip(open) {
            if matches!(cell, Cell::Expander(..)) {
                disclosure::set_open(tree, *open);
            }
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for RowGroupCore<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.cells
            .iter()
            .map(|cell| Tree::new(cell.widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let widgets: Vec<_> = self.cells.iter().map(Cell::widget).collect();
        tree.diff_children(&widgets);

        // The grid may have changed under panels left open, e.g. to fewer
        // columns: later panels win over the ones they now overlap.
        let footprints = self.footprints();
        let was_open = self.open(tree);
        let mut open = vec![false; was_open.len()];
        for (index, _) in was_open.iter().enumerate().filter(|(_, open)| **open) {
            open_evicting(&mut open, &footprints, index);
        }
        self.set_open(tree, &open);
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
        let width = limits.max().width;
        let max_height = limits.max().height;
        let cell_width = cell_width(width, self.columns);
        let loose = layout::Limits::new(
            Size::new(cell_width, 0.0),
            Size::new(cell_width, max_height),
        );
        let mut nodes = Vec::with_capacity(self.cells.len());
        let line_count = self.cells.len().div_ceil(self.columns);
        let mut top = 0.0;

        for line in 0..line_count {
            let cells = line * self.columns..((line + 1) * self.columns).min(self.cells.len());
            // Every header on a line is as tall as the tallest.
            let header_height = cells
                .clone()
                .map(|index| {
                    let tree = &mut tree.children[index];
                    match &mut self.cells[index] {
                        Cell::Row(row) => row.as_widget_mut().layout(tree, renderer, &loose),
                        Cell::Expander(disclosure, _) => {
                            disclosure.layout_header(tree, renderer, &loose)
                        }
                    }
                    .size()
                    .height
                })
                .fold(0.0, f32::max);
            let fixed = layout::Limits::new(
                Size::new(cell_width, header_height),
                Size::new(cell_width, header_height),
            );
            let body_top = top + header_height + spacing::SM;
            let mut body_height: Option<f32> = None;

            for (column, index) in cells.enumerate() {
                let tree = &mut tree.children[index];
                let position = Point::new(column as f32 * (cell_width + spacing::SM), top);
                nodes.push(match &mut self.cells[index] {
                    Cell::Row(row) => row
                        .as_widget_mut()
                        .layout(tree, renderer, &fixed)
                        .move_to(position),
                    Cell::Expander(disclosure, footprint) => {
                        let header = disclosure
                            .layout_header(tree, renderer, &fixed)
                            .move_to(position);
                        let panel = disclosure::is_open(tree).then(|| {
                            let panel = disclosure
                                .layout_panel(
                                    tree,
                                    renderer,
                                    footprint_width(*footprint, cell_width),
                                    max_height,
                                )
                                .move_to(Point::new(
                                    footprint.start as f32 * (cell_width + spacing::SM),
                                    body_top,
                                ));
                            body_height = Some(body_height.unwrap_or(0.0).max(panel.size().height));
                            panel
                        });
                        disclosure::place(header, panel)
                    }
                });
            }

            top += header_height;
            if let Some(body_height) = body_height {
                top += spacing::SM + body_height;
            }
            if line + 1 < line_count {
                top += spacing::SM;
            }
        }

        layout::Node::with_children(Size::new(width, top), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let layouts: Vec<_> = layout.children().collect();
        let columns = self.columns;

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            // In reading order: a line's headers, then the panels under it.
            for ((cells, trees), layouts) in self
                .cells
                .chunks_mut(columns)
                .zip(tree.children.chunks_mut(columns))
                .zip(layouts.chunks(columns))
            {
                for ((cell, tree), layout) in cells.iter_mut().zip(trees.iter_mut()).zip(layouts) {
                    match cell {
                        Cell::Row(row) => {
                            row.as_widget_mut()
                                .operate(tree, *layout, renderer, operation);
                        }
                        Cell::Expander(disclosure, _) => {
                            disclosure.operate_header(tree, *layout, renderer, operation);
                        }
                    }
                }
                for ((cell, tree), layout) in cells.iter_mut().zip(trees.iter_mut()).zip(layouts) {
                    if let Cell::Expander(disclosure, _) = cell {
                        disclosure.operate_panel(tree, *layout, renderer, operation);
                    }
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
        let was_open = self.open(tree);

        for ((cell, tree), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        let mut open = self.open(tree);
        if let Some(opened) = (0..open.len()).find(|&index| open[index] && !was_open[index]) {
            open_evicting(&mut open, &self.footprints(), opened);
            self.set_open(tree, &open);
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
        self.cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((cell, tree), layout)| {
                cell.widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        for ((cell, tree), layout) in self.cells.iter().zip(&tree.children).zip(layout.children()) {
            cell.widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
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
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((cell, tree), layout)| {
                cell.widget_mut()
                    .overlay(tree, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

fn cell_width(width: f32, columns: usize) -> f32 {
    (width - spacing::SM * (columns.saturating_sub(1) as f32)).max(0.0) / columns as f32
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
    span as f32 * cell_width + span.saturating_sub(1) as f32 * spacing::SM
}

/// Opens the panel of cell `target`, closing the open ones it overlaps.
fn open_evicting(open: &mut [bool], footprints: &[Option<Footprint>], target: usize) {
    let Some(target_footprint) = footprints[target] else {
        return;
    };

    for (open, footprint) in open.iter_mut().zip(footprints) {
        if footprint.is_some_and(|footprint| footprint.overlaps(target_footprint)) {
            *open = false;
        }
    }

    open[target] = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_keeps_disjoint_panels_and_evicts_overlaps() {
        let footprints =
            [(0, 1), (1, 2), (2, 3)].map(|(header, span)| Some(footprint(header, span, 3)));
        let mut open = [false; 3];

        open_evicting(&mut open, &footprints, 0);
        open_evicting(&mut open, &footprints, 1);
        assert!(open[0] && open[1]);

        open_evicting(&mut open, &footprints, 2);
        assert_eq!(open, [false, false, true]);
    }

    #[test]
    fn column_count_is_never_zero() {
        let group = RowGroup::<'static, ()>::new().columns(0);

        assert_eq!(group.columns, 1);
    }
}
