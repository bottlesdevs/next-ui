use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Padding, Point, Rectangle, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    widget::{Space, button, column, row, svg, text, text::Fragment},
};

use crate::icons::Icon;

use super::{
    control::{Control, State},
    spacing,
    text::TextExt as _,
};

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

pub(super) fn item<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    selected: bool,
    keyboard_highlighted: impl Fn() -> bool + 'a,
) -> Element<'a, Message> {
    Control::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press_maybe(on_press)
        .selected(selected)
        .style(move |theme, mut state| {
            state.keyboard_highlighted = keyboard_highlighted();
            row_style(theme, state)
        })
        .into()
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
    .style(row_style)
    .into()
}

fn row_style(theme: &Theme, state: State) -> button::Style {
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

/// Menu rows measured intrinsically, then laid out at their widest shared width.
pub(super) struct MenuRows<'a, Message, Renderer = iced::Renderer> {
    children: Vec<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Renderer> MenuRows<'a, Message, Renderer> {
    pub(super) fn new(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            children: children.into_iter().collect(),
        }
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for MenuRows<'_, Message, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let requested_padding = Padding::new(spacing::MD);
        let padded_size = Size::new(requested_padding.x(), requested_padding.y());
        let inner_limits = limits.shrink(padded_size);
        let probe_limits =
            layout::Limits::with_compression(Size::ZERO, inner_limits.max(), Size::new(true, true));
        let intrinsic_width = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, tree)| {
                child
                    .as_widget_mut()
                    .layout(tree, renderer, &probe_limits)
                    .size()
                    .width
            })
            .fold(0.0, f32::max);
        let width = limits
            .resolve(
                Length::Shrink,
                Length::Shrink,
                Size::new(intrinsic_width + requested_padding.x(), 0.0),
            )
            .width;
        let row_width = (width - requested_padding.x()).max(0.0);
        let row_limits = layout::Limits::with_compression(
            Size::new(row_width, 0.0),
            Size::new(row_width, inner_limits.max().height),
            Size::new(false, true),
        );
        let mut height = 0.0;
        let mut children: Vec<layout::Node> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, tree)| {
                let node = child
                    .as_widget_mut()
                    .layout(tree, renderer, &row_limits)
                    .move_to(Point::new(0.0, height));
                height += node.size().height;
                node
            })
            .collect();
        let size = limits.resolve(
            Length::Shrink,
            Length::Shrink,
            Size::new(width, height + requested_padding.y()),
        );
        let padding = requested_padding.fit(Size::new(row_width, height), size);

        for child in &mut children {
            child.translate_mut(Vector::new(padding.left, padding.top));
        }

        layout::Node::with_children(size, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
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
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .filter(|(_, layout)| layout.bounds().intersects(viewport))
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
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

impl<'a, Message, Renderer> From<MenuRows<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(rows: MenuRows<'a, Message, Renderer>) -> Self {
        Element::new(rows)
    }
}

#[cfg(test)]
mod tests {
    use iced::{
        Fill,
        advanced::{Widget, layout, widget::Tree},
        widget::{Space, container},
    };

    use super::*;

    #[test]
    fn rows_share_the_widest_intrinsic_width() {
        let children: Vec<Element<'static, (), Theme, ()>> = vec![
            container(Space::new().width(60).height(10))
                .width(Fill)
                .into(),
            container(Space::new().width(100).height(20))
                .width(Fill)
                .into(),
        ];
        let mut rows = MenuRows::new(children);
        let mut tree = Tree::new(&rows as &dyn Widget<(), Theme, ()>);
        let limits = layout::Limits::with_compression(
            Size::ZERO,
            Size::new(200.0, 200.0),
            Size::new(true, true),
        );

        let node = rows.layout(&mut tree, &(), &limits);

        assert_eq!(node.size(), Size::new(136.0, 66.0));
        assert_eq!(node.children()[0].size().width, 100.0);
        assert_eq!(node.children()[1].size().width, 100.0);
        assert_eq!(node.children()[0].bounds().x, 18.0);
        assert_eq!(node.children()[0].bounds().y, 18.0);
        assert_eq!(node.children()[1].bounds().y, 28.0);
    }
}
