use iced::{
    Length, Rectangle, Size,
    advanced::{
        Layout, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, tree},
    },
};

use crate::{Element, Theme};

use super::inert::inert;

/// Shows `content` as usual while `enabled`, and otherwise disabled as a
/// whole: it takes no input or focus, and draws with [`Theme::disabled`].
///
/// One switch for a group of controls, so none of them needs telling.
pub fn sensitive<'a, Message: 'a>(
    enabled: bool,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    if enabled {
        content.into()
    } else {
        inert(Element::new(Disabled {
            content: content.into(),
        }))
    }
}

/// Draws `content` with [`Theme::disabled`]. Input never reaches it, since
/// it only sits under [`inert`].
///
/// The content's tree is kept as is, so switching `enabled` loses no state.
struct Disabled<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Disabled<'_, Message> {
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

    fn size(&self) -> Size<Length> {
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
        let theme = theme.disabled();
        let style = renderer::Style {
            text_color: theme.colors().text,
        };

        self.content
            .as_widget()
            .draw(tree, renderer, &theme, &style, layout, cursor, viewport);
    }
}
