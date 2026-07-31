use iced::{
    Element, Fill, Padding, Point, Rectangle, Renderer, Theme, mouse,
    widget::{Row, Space, canvas, column, container, stack, text},
};

use super::{
    expander_row::{ExpanderParts, ExpanderRow},
    list_row::ListRow,
    text::TextExt as _,
};

const HEADER_HEIGHT: f32 = 84.0;
const GAP: f32 = 16.0;
const RADIUS: f32 = 8.0;
const CONTENT_PADDING: f32 = 18.0;

pub struct RowGroup<'a, Message> {
    title: Option<&'a str>,
    description: Option<&'a str>,
    columns: usize,
    enabled: bool,
    entries: Vec<RowGroupEntry<'a, Message>>,
}

struct Expansion<'a, Message> {
    expanded: bool,
    content: Option<Element<'a, Message>>,
}

#[doc(hidden)]
pub struct RowGroupEntry<'a, Message> {
    row: ListRow<'a, Message>,
    expansion: Option<Expansion<'a, Message>>,
}

impl<'a, Message, T> From<T> for RowGroupEntry<'a, Message>
where
    T: Into<ListRow<'a, Message>>,
{
    fn from(row: T) -> Self {
        Self {
            row: row.into(),
            expansion: None,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for RowGroupEntry<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        let ExpanderParts {
            header,
            expanded,
            content,
        } = expander.into_parts();

        Self {
            row: header,
            expansion: Some(Expansion { expanded, content }),
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
        let mut rows = column![].spacing(GAP);
        let mut entries = group.entries.into_iter();

        loop {
            let line: Vec<_> = entries.by_ref().take(group.columns).collect();

            if line.is_empty() {
                break;
            }

            rows = rows.push(group_line(line, group.columns, group.enabled));
        }

        let mut content = column![].spacing(GAP);

        if group.title.is_some() || group.description.is_some() {
            let mut heading = column![].spacing(4);

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
) -> Element<'a, Message> {
    let mut headers = Vec::with_capacity(columns);
    let mut active_expansion = None;

    for (index, entry) in entries.into_iter().enumerate() {
        if let Some(expansion) = entry.expansion
            && expansion.expanded
            && expansion.content.is_some()
        {
            assert!(
                active_expansion.is_none(),
                "RowGroup supports one expanded row per grid line"
            );
            active_expansion = Some((index, expansion));
        }

        headers.push(Element::from(
            entry.row.height(HEADER_HEIGHT).enabled(enabled),
        ));
    }

    headers.extend(
        (headers.len()..columns).map(|_| Space::new().width(Fill).height(HEADER_HEIGHT).into()),
    );

    let headers = Row::with_children(headers).width(Fill).spacing(GAP);
    let Some((expanded_index, expansion)) = active_expansion else {
        return headers.into();
    };
    let content = expansion.content.expect("checked above");
    let body = ListRow::new(Space::new())
        .height(0)
        .padding(0)
        .raised(true)
        .content(
            container(content)
                .width(Fill)
                .padding(Padding::from(CONTENT_PADDING)),
        );

    stack![column![headers, Space::new().height(GAP), body]]
        .push_under(
            canvas::Canvas::new(GroupSurface {
                columns,
                expanded_index,
            })
            .width(Fill)
            .height(Fill),
        )
        .into()
}

#[derive(Debug, Clone, Copy)]
struct GroupSurface {
    columns: usize,
    expanded_index: usize,
}

impl<Message> canvas::Program<Message> for GroupSurface {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (branch_left, branch_right) =
            branch_bounds(bounds.width, self.columns, self.expanded_index);
        let body_top = HEADER_HEIGHT + GAP;
        let width = bounds.width;
        let height = bounds.height;

        let path = canvas::Path::new(|path| {
            path.move_to(Point::new(branch_left + RADIUS, 0.0));
            path.line_to(Point::new(branch_right - RADIUS, 0.0));

            if branch_right < width {
                path.quadratic_curve_to(
                    Point::new(branch_right, 0.0),
                    Point::new(branch_right, RADIUS),
                );
                path.line_to(Point::new(branch_right, body_top - RADIUS));
                path.quadratic_curve_to(
                    Point::new(branch_right, body_top),
                    Point::new(branch_right + RADIUS, body_top),
                );
                path.line_to(Point::new(width - RADIUS, body_top));
                path.quadratic_curve_to(
                    Point::new(width, body_top),
                    Point::new(width, body_top + RADIUS),
                );
            } else {
                path.quadratic_curve_to(Point::new(width, 0.0), Point::new(width, RADIUS));
            }

            path.line_to(Point::new(width, height - RADIUS));
            path.quadratic_curve_to(
                Point::new(width, height),
                Point::new(width - RADIUS, height),
            );
            path.line_to(Point::new(RADIUS, height));
            path.quadratic_curve_to(Point::new(0.0, height), Point::new(0.0, height - RADIUS));

            if branch_left > 0.0 {
                path.line_to(Point::new(0.0, body_top + RADIUS));
                path.quadratic_curve_to(Point::new(0.0, body_top), Point::new(RADIUS, body_top));
                path.line_to(Point::new(branch_left - RADIUS, body_top));
                path.quadratic_curve_to(
                    Point::new(branch_left, body_top),
                    Point::new(branch_left, body_top - RADIUS),
                );
                path.line_to(Point::new(branch_left, RADIUS));
                path.quadratic_curve_to(
                    Point::new(branch_left, 0.0),
                    Point::new(branch_left + RADIUS, 0.0),
                );
            } else {
                path.line_to(Point::new(0.0, RADIUS));
                path.quadratic_curve_to(Point::ORIGIN, Point::new(RADIUS, 0.0));
            }

            path.close();
        });

        frame.fill(&path, theme.extended_palette().background.neutral.color);
        vec![frame.into_geometry()]
    }
}

fn branch_bounds(width: f32, columns: usize, index: usize) -> (f32, f32) {
    let column_width = (width - GAP * (columns.saturating_sub(1) as f32)) / columns as f32;
    let left = index as f32 * (column_width + GAP);
    (left, left + column_width)
}

#[cfg(test)]
mod tests {
    use crate::components::{expander_row::ExpanderRow, switcher_row::SwitcherRow};

    use super::{RowGroup, branch_bounds};

    #[test]
    fn branch_tracks_its_column() {
        assert_eq!(branch_bounds(332.0, 3, 0), (0.0, 100.0));
        assert_eq!(branch_bounds(332.0, 3, 1), (116.0, 216.0));
        assert_eq!(branch_bounds(332.0, 3, 2), (232.0, 332.0));
    }

    #[test]
    fn add_accepts_rows_and_expanders() {
        let group = RowGroup::new()
            .columns(3)
            .add(SwitcherRow::new(false, |_| ()))
            .add(ExpanderRow::new(()));

        assert_eq!(group.columns, 3);
        assert!(group.entries[0].expansion.is_none());
        assert!(group.entries[1].expansion.is_some());
    }
}
