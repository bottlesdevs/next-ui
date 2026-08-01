use iced::{
    Border, ContentFit, Element, Length, Point, Rectangle, Size, Theme,
    advanced::{Layout, Widget, layout, mouse, renderer, svg::Renderer as _, widget::Tree},
    widget::button,
};

use super::{
    list_row::{HoverTone, ListRow},
    pressable::{Pressable, SharedFlag, Status},
    row_group::{RowGroupEntry, standalone_expander},
    spacing,
};

pub struct ExpanderRow<'a, Message> {
    header: Header<'a, Message>,
    columns: usize,
    content: Vec<ListRow<'a, Message>>,
    content_enabled: bool,
    enabled: bool,
}

pub(crate) enum Header<'a, Message> {
    Labels {
        title: &'a str,
        description: &'a str,
    },
    Custom(ListRow<'a, Message>),
}

pub(crate) struct ExpanderParts<'a, Message> {
    pub header: Header<'a, Message>,
    pub columns: usize,
    pub content: Vec<ListRow<'a, Message>>,
    pub content_enabled: bool,
    pub enabled: bool,
}

impl<'a, Message> ExpanderRow<'a, Message> {
    pub fn new(title: &'a str) -> Self {
        Self {
            header: Header::Labels {
                title,
                description: "",
            },
            columns: 1,
            content: Vec::new(),
            content_enabled: true,
            enabled: true,
        }
    }

    pub fn with_header(header: impl Into<ListRow<'a, Message>>) -> Self {
        Self {
            header: Header::Custom(header.into()),
            ..Self::new("")
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        if let Header::Labels {
            description: current,
            ..
        } = &mut self.header
        {
            *current = description;
        }

        self
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn add(mut self, row: impl Into<ListRow<'a, Message>>) -> Self {
        let mut row = row.into();
        row.set_hover_tone(HoverTone::Strong);
        self.content.push(row);
        self
    }

    pub fn content_enabled(mut self, enabled: bool) -> Self {
        self.content_enabled = enabled;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub(crate) fn into_parts(self) -> ExpanderParts<'a, Message> {
        ExpanderParts {
            header: self.header,
            columns: self.columns,
            content: self.content,
            content_enabled: self.content_enabled,
            enabled: self.enabled,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ExpanderRow<'a, Message>> for Element<'a, Message> {
    fn from(expander: ExpanderRow<'a, Message>) -> Self {
        standalone_expander(RowGroupEntry::from(expander))
    }
}

pub(crate) fn control<'a, Message: Clone + 'a>(
    header: Header<'a, Message>,
    enabled: bool,
    activated: SharedFlag,
    expanded: SharedFlag,
) -> ListRow<'a, Message> {
    let caret = || DynamicCaret::new(expanded.clone());

    match header {
        Header::Labels { title, description } => {
            ListRow::new(super::list_row::labels(title, description))
                .trailing(caret())
                .on_activate(activated)
        }
        Header::Custom(header) => header.prepend_trailing(
            Pressable::new(caret())
                .padding(spacing::XS)
                .on_activate(activated)
                .style(caret_style),
        ),
    }
    .raised_when(expanded)
    .enabled(enabled)
}

pub(crate) fn passive<'a, Message: 'a>(
    header: Header<'a, Message>,
    enabled: bool,
) -> ListRow<'a, Message> {
    match header {
        Header::Labels { title, description } => {
            ListRow::new(super::list_row::labels(title, description))
        }
        Header::Custom(header) => header,
    }
    .enabled(enabled)
}

fn caret_style(theme: &Theme, _status: Status) -> button::Style {
    button::Style {
        text_color: theme.palette().text,
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}

struct DynamicCaret {
    expanded: SharedFlag,
}

impl DynamicCaret {
    fn new(expanded: SharedFlag) -> Self {
        Self { expanded }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for DynamicCaret {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(20.0), Length::Fixed(20.0))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, 20.0, 20.0)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let handle = crate::icons::Icon::DownCaret.handle();
        let Size { width, height } = renderer.measure_svg(&handle);
        let drawing_bounds = contained_bounds(bounds, Size::new(width as f32, height as f32));

        renderer.draw_svg(
            iced::advanced::svg::Svg {
                handle,
                color: None,
                rotation: if self.expanded.get() {
                    std::f32::consts::PI.into()
                } else {
                    0.0.into()
                },
                opacity: 1.0,
            },
            drawing_bounds,
            bounds,
        );
    }
}

fn contained_bounds(bounds: Rectangle, image_size: Size) -> Rectangle {
    let size = ContentFit::Contain.fit(image_size, bounds.size());

    Rectangle::new(
        Point::new(
            bounds.center_x() - size.width / 2.0,
            bounds.center_y() - size.height / 2.0,
        ),
        size,
    )
}

impl<'a, Message: 'a> From<DynamicCaret> for Element<'a, Message> {
    fn from(caret: DynamicCaret) -> Self {
        Element::new(caret)
    }
}
