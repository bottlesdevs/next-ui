use crate::theme;
use iced::{
    Element, Event, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    theme::palette::Pair,
    widget::container,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Panel,
    Card,
    Overlay,
}

impl Kind {
    fn colors(self, theme: &Theme) -> Pair {
        let background = theme.extended_palette().background;

        match self {
            Self::Panel | Self::Overlay => background.neutral,
            Self::Card => background.weak,
        }
    }

    fn style(self, theme: &Theme) -> container::Style {
        theme::surface(self.colors(theme))
    }

    pub(super) fn draw_background(
        self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
    ) -> Theme {
        container::draw_background(renderer, &self.style(theme), bounds);
        self.scoped_theme(theme)
    }

    pub(super) fn scoped_theme(self, theme: &Theme) -> Theme {
        let colors = self.colors(theme);
        let mut palette = theme.palette();
        let mut extended = *theme.extended_palette();

        palette.background = colors.color;
        palette.text = colors.text;
        extended.background.base = colors;
        extended.background.strong = match self {
            Self::Panel => extended.background.strong,
            Self::Card => extended.background.neutral,
            Self::Overlay => extended.background.stronger,
        };

        Theme::custom_with_fn(self.name(), palette, |_| extended)
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Panel => "Bottles Next Panel",
            Self::Card => "Bottles Next Card",
            Self::Overlay => "Bottles Next Overlay",
        }
    }
}

pub(super) struct Surface<'a, Message> {
    kind: Kind,
    content: Element<'a, Message>,
}

impl<'a, Message> Surface<'a, Message> {
    pub(super) fn new(kind: Kind, content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            kind,
            content: content.into(),
        }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Surface<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if layout.bounds().intersection(viewport).is_none() {
            return;
        }

        let scoped_theme = self.kind.draw_background(renderer, theme, layout.bounds());

        self.content.as_widget().draw(
            tree,
            renderer,
            &scoped_theme,
            &renderer::Style {
                text_color: scoped_theme.palette().text,
            },
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
            .map(|content| scoped_overlay(self.kind, content))
    }
}

impl<'a, Message: 'a> From<Surface<'a, Message>> for Element<'a, Message> {
    fn from(surface: Surface<'a, Message>) -> Self {
        Element::new(surface)
    }
}

struct ScopedOverlay<'a, Message> {
    kind: Kind,
    content: overlay::Element<'a, Message, Theme, iced::Renderer>,
}

pub(super) fn scoped_overlay<'a, Message: 'a>(
    kind: Kind,
    content: overlay::Element<'a, Message, Theme, iced::Renderer>,
) -> overlay::Element<'a, Message, Theme, iced::Renderer> {
    overlay::Element::new(Box::new(ScopedOverlay { kind, content }))
}

impl<Message> overlay::Overlay<Message, Theme, iced::Renderer> for ScopedOverlay<'_, Message> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        self.content.as_overlay_mut().layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let scoped_theme = self.kind.scoped_theme(theme);

        self.content.as_overlay().draw(
            renderer,
            &scoped_theme,
            &renderer::Style {
                text_color: scoped_theme.palette().text,
            },
            layout,
            cursor,
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
    ) {
        self.content
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_overlay()
            .mouse_interaction(layout, cursor, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.content
            .as_overlay_mut()
            .overlay(layout, renderer)
            .map(|content| scoped_overlay(self.kind, content))
    }
}
