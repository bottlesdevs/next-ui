use iced::{
    Background, Border, Element, Event, Length, Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};

pub(crate) struct RowSurface<'a, Message, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    raised: bool,
}

impl<'a, Message, Renderer> RowSurface<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    pub(crate) fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            raised: false,
        }
    }

    pub(crate) fn raised(mut self, raised: bool) -> Self {
        self.raised = raised;
        self
    }
}

#[derive(Default)]
struct State {
    hovered: bool,
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for RowSurface<'_, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        let hovered = cursor.is_over(layout.bounds());

        if state.hovered != hovered {
            state.hovered = hovered;
            shell.request_redraw();
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: Border::default().rounded(8),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(surface_color(
                theme,
                self.raised,
                tree.state.downcast_ref::<State>().hovered,
            )),
        );

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a, Renderer: renderer::Renderer + 'a> From<RowSurface<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(surface: RowSurface<'a, Message, Renderer>) -> Self {
        Element::new(surface)
    }
}

fn surface_color(theme: &Theme, raised: bool, hovered: bool) -> iced::Color {
    if raised || hovered {
        theme.extended_palette().background.neutral.color
    } else {
        theme.extended_palette().background.weak.color
    }
}

#[cfg(test)]
mod tests {
    use crate::theme;

    use super::surface_color;

    #[test]
    fn hover_uses_the_raised_row_color() {
        let theme = theme::theme();

        assert_eq!(surface_color(&theme, false, false), theme::SURFACE);
        assert_eq!(surface_color(&theme, false, true), theme::BORDER);
        assert_eq!(surface_color(&theme, true, false), theme::BORDER);
    }
}
