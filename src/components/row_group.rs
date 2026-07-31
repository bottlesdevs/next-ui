use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    widget::{column, text},
};

use super::{
    expander_row::{ExpanderParts, ExpanderRow},
    list_row::ListRow,
    text::TextExt as _,
};

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

pub(crate) fn group_line<'a, Message: Clone + 'a>(
    entries: Vec<RowGroupEntry<'a, Message>>,
    columns: usize,
    enabled: bool,
) -> Element<'a, Message> {
    let mut headers = Vec::with_capacity(entries.len());
    let mut expanded = Vec::new();

    for (index, entry) in entries.into_iter().enumerate() {
        if let Some(expansion) = entry.expansion
            && expansion.expanded
            && let Some(content) = expansion.content
        {
            expanded.push((index, content));
        }

        headers.push(Element::from(entry.row.enabled(enabled)));
    }

    let header_count = headers.len();
    let expanded_headers = expanded.iter().map(|(index, _)| *index).collect();
    headers.extend(expanded.into_iter().map(|(_, content)| content));

    Element::new(GroupLine {
        children: headers,
        header_count,
        expanded_headers,
        columns: columns.max(1),
    })
}

struct GroupLine<'a, Message> {
    children: Vec<Element<'a, Message>>,
    header_count: usize,
    expanded_headers: Vec<usize>,
    columns: usize,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for GroupLine<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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
        let mut children = Vec::with_capacity(tree.children.len());

        for (index, (header, tree)) in self.children[..self.header_count]
            .iter_mut()
            .zip(&mut tree.children[..self.header_count])
            .enumerate()
        {
            children.push(
                header
                    .as_widget_mut()
                    .layout(tree, renderer, &header_limits)
                    .move_to(Point::new(index as f32 * (cell_width + GAP), 0.0)),
            );
        }

        if self.expanded_headers.is_empty() {
            return layout::Node::with_children(Size::new(width, header_height), children);
        }

        let body_top = header_height + GAP;
        let inner_width = (width - CONTENT_PADDING * 2.0).max(0.0);
        let content_limits = layout::Limits::new(
            Size::new(inner_width, 0.0),
            Size::new(inner_width, limits.max().height),
        );
        let mut y = body_top + CONTENT_PADDING;

        for (content_index, content) in self.children[self.header_count..].iter_mut().enumerate() {
            let tree = &mut tree.children[self.header_count + content_index];
            let node = content
                .as_widget_mut()
                .layout(tree, renderer, &content_limits)
                .move_to(Point::new(CONTENT_PADDING, y));
            y += node.size().height + GAP;
            children.push(node);
        }

        y += CONTENT_PADDING - GAP;
        layout::Node::with_children(Size::new(width, y), children)
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
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, tree), layout)| {
                    child
                        .as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                });
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
        self.children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .for_each(|((child, tree), layout)| {
                child.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                );
            });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
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
        let children: Vec<_> = layout.children().collect();

        if !self.expanded_headers.is_empty() {
            let bounds = layout.bounds();
            let header_bottom = children[..self.header_count]
                .iter()
                .map(|header| header.bounds().y + header.bounds().height)
                .fold(bounds.y, f32::max);
            let body_top = header_bottom + GAP;

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        y: body_top,
                        height: (bounds.y + bounds.height - body_top).max(0.0),
                        ..bounds
                    },
                    border: Border::default().rounded(RADIUS),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(theme.extended_palette().background.neutral.color),
            );

            for index in &self.expanded_headers {
                let header = children[*index].bounds();
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: header.x,
                            y: header.y + header.height - RADIUS,
                            width: header.width,
                            height: GAP + RADIUS * 2.0,
                        },
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: true,
                    },
                    Background::Color(theme.extended_palette().background.neutral.color),
                );
            }
        }

        self.children
            .iter()
            .zip(&tree.children)
            .zip(children)
            .for_each(|((child, tree), layout)| {
                child
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, viewport);
            });
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn cell_width(width: f32, columns: usize) -> f32 {
    (width - GAP * (columns.saturating_sub(1) as f32)) / columns as f32
}

#[cfg(test)]
mod tests {
    use iced::Element;

    use crate::components::{
        action_row::{ActionRow, ActionRowState},
        expander_row::ExpanderRow,
        switcher_row::SwitcherRow,
    };

    use super::{RowGroup, cell_width};

    #[test]
    fn cells_follow_the_requested_column_count() {
        assert_eq!(cell_width(332.0, 3), 100.0);
    }

    #[test]
    fn a_line_accepts_multiple_open_expanders_without_panicking() {
        let group = RowGroup::new()
            .columns(3)
            .add(SwitcherRow::new("Switch", false, |_| ()))
            .add(
                ExpanderRow::new(())
                    .expanded(true)
                    .add(ActionRow::new("First", ActionRowState::Ready(()))),
            )
            .add(
                ExpanderRow::new(())
                    .expanded(true)
                    .add(ActionRow::new("Second", ActionRowState::Ready(()))),
            );

        let _: Element<'_, ()> = group.into();
    }
}
