use iced::{
    Background, Border, Center, Fill, Length, Point, Size,
    advanced::{layout, widget::Tree},
    widget::{Row, Space, container},
};

use crate::{Element, Theme, icons::Icon};

use super::{
    arranged::Arrangement,
    button::{Button, ButtonKind},
    header,
    natural::natural_layout,
    radius, spacing,
    window_handle::WindowHandle,
};

/// Builds a header-sized icon button; the caller supplies its action.
pub fn header_button<Message>(label: &str, icon: Icon) -> Button<'_, Message> {
    Button::icon_only(label, icon)
        .diameter(header::CONTROL)
        .icon_size(header::ICON)
        .kind(ButtonKind::Flat)
}

pub struct HeaderBar<'a, Message> {
    start: Vec<Element<'a, Message>>,
    middle: Option<Element<'a, Message>>,
    end: Vec<Element<'a, Message>>,
    transparent: bool,
}

impl<Message> Default for HeaderBar<'_, Message> {
    fn default() -> Self {
        Self {
            start: Vec::new(),
            middle: None,
            end: Vec::new(),
            transparent: false,
        }
    }
}

impl<'a, Message> HeaderBar<'a, Message> {
    pub fn start(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.start.push(content.into());
        self
    }

    /// Sets the one widget centred in the bar, e.g. a title, tabs or a
    /// search entry. It gets the space between the sides, so one that fills
    /// grows up to its own maximum width.
    pub fn middle(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.middle = Some(content.into());
        self
    }

    pub fn end(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.end.push(content.into());
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
}

impl<'a, Message: 'a> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(header: HeaderBar<'a, Message>) -> Self {
        let HeaderBar {
            start,
            middle,
            end,
            transparent,
        } = header;

        let content = center_box(
            container(section(start)).clip(true),
            middle.unwrap_or_else(|| Space::new().into()),
            container(section(end)).clip(true),
        );

        let mut content = container(content)
            .width(Fill)
            .height(header::HEIGHT)
            .padding([0.0, header::INSET]);
        if !transparent {
            content = content.style(style);
        }

        WindowHandle::new(content).into()
    }
}

fn section<'a, Message: 'a>(children: Vec<Element<'a, Message>>) -> Row<'a, Message, Theme> {
    Row::with_children(children)
        .spacing(spacing::SM)
        .align_y(Center)
}

fn style(current_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(current_theme.colors().chrome)),
        border: Border::default().rounded(iced::border::top(radius::ROW)),
        ..container::Style::default()
    }
}

/// Lays out a header's `start`, `middle` and `end`, the only children a
/// [`CenterBox`] is ever given.
fn center_box<'a, Message: 'a>(
    start: impl Into<Element<'a, Message>>,
    middle: impl Into<Element<'a, Message>>,
    end: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    CenterBox.arrange([start.into(), middle.into(), end.into()])
}

/// Lays out a header's start, middle and end. Each side is as wide as the
/// wider of the two wants, so the middle sits at the centre of the bar
/// whatever either side holds; the middle gets everything between them, so
/// what in it fills, e.g. a search entry, grows up to its own maximum, and
/// what doesn't keeps its natural width, centred.
struct CenterBox;

impl Arrangement for CenterBox {
    fn size<Message>(&self, _: &[Element<'_, Message>]) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout<Message>(
        &self,
        children: &mut [Element<'_, Message>],
        trees: &mut [Tree],
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bar = limits.max();
        // `center_box` gives every center box exactly these three.
        let ([start, middle, end], [start_tree, middle_tree, end_tree]) = (children, trees) else {
            unreachable!("a center box has a start, a middle and an end")
        };

        let side = natural_layout(start, start_tree, renderer, bar)
            .size()
            .width
            .max(natural_layout(end, end_tree, renderer, bar).size().width);
        let room = (bar.width - 2.0 * (side + spacing::SM)).max(0.0);
        let middle = middle.as_widget_mut().layout(
            middle_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(room, bar.height)),
        );
        let middle_x = (bar.width - middle.size().width) / 2.0;

        // Each side takes the space beside the middle, up to its edge.
        let beside = (middle_x - spacing::SM).max(0.0);
        let side_limits = layout::Limits::new(Size::ZERO, Size::new(beside, bar.height));
        let start = start
            .as_widget_mut()
            .layout(start_tree, renderer, &side_limits);
        let end = end.as_widget_mut().layout(end_tree, renderer, &side_limits);

        let centred = |node: layout::Node, x: f32| {
            let y = (bar.height - node.size().height) / 2.0;
            node.move_to(Point::new(x, y))
        };
        let end_x = bar.width - end.size().width;
        layout::Node::with_children(
            bar,
            vec![
                centred(start, 0.0),
                centred(middle, middle_x),
                centred(end, end_x),
            ],
        )
    }
}
