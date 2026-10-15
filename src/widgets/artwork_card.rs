use iced::{
    Center, ContentFit, Element, Event, Fill, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
    widget::{Space, column, container, image, row, stack, text},
};

use crate::{icons::Icon, widgets::button::ButtonKind};

use super::{button::Button, card::Card, spacing, text::TextExt as _};

const ACTION_DIAMETER: f32 = 40.0;
const PRIMARY_ACTION_DIAMETER: f32 = 48.0;

pub struct CardAction<'a, Message> {
    label: &'a str,
    icon: Icon,
    message: Option<Message>,
    loading: bool,
}

impl<'a, Message> CardAction<'a, Message> {
    pub fn new(label: &'a str, icon: Icon) -> Self {
        Self {
            label,
            icon,
            message: None,
            loading: false,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.message = message;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    fn button(self) -> Button<'a, Message> {
        Button::icon_only(self.label, self.icon)
            .on_press_maybe(self.message)
            .loading(self.loading)
    }
}

pub struct ArtworkCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    menu: Option<CardAction<'a, Message>>,
    secondary: Option<CardAction<'a, Message>>,
    primary: Option<CardAction<'a, Message>>,
}

impl<'a, Message> ArtworkCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: &'a str) -> Self {
        Self {
            title,
            subtitle,
            banner: None,
            menu: None,
            secondary: None,
            primary: None,
        }
    }

    pub fn menu(mut self, menu: CardAction<'a, Message>) -> Self {
        self.menu = Some(menu);
        self
    }

    pub fn secondary(mut self, secondary: CardAction<'a, Message>) -> Self {
        self.secondary = Some(secondary);
        self
    }

    pub fn primary(mut self, primary: CardAction<'a, Message>) -> Self {
        self.primary = Some(primary);
        self
    }

    pub fn banner(mut self, banner: impl Into<image::Handle>) -> Self {
        self.banner = Some(banner.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<ArtworkCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ArtworkCard<'a, Message>) -> Self {
        let mut menu = row![Space::new().width(Fill)].width(Fill);

        if let Some(action) = card.menu {
            menu = menu.push(
                action
                    .button()
                    .kind(ButtonKind::Transparent)
                    .diameter(16.0)
                    .icon_size(16.0),
            );
        }

        let mut main_actions = row![Space::new().width(Fill)]
            .spacing(spacing::XS)
            .align_y(Center)
            .width(Fill);

        if let Some(action) = card.secondary {
            main_actions = main_actions.push(action.button().diameter(ACTION_DIAMETER));
        }

        if let Some(action) = card.primary {
            main_actions = main_actions.push(
                action
                    .button()
                    .diameter(PRIMARY_ACTION_DIAMETER)
                    .kind(ButtonKind::Primary),
            );
        }

        let menu = container(menu)
            .padding(spacing::SM)
            .width(Fill)
            .height(Fill);
        let artwork: Element<'a, Message> = match card.banner {
            Some(handle) => image(handle)
                .width(Fill)
                .height(Fill)
                .content_fit(ContentFit::Cover)
                .border_radius(6)
                .into(),
            None => Space::new().width(Fill).height(Fill).into(),
        };
        let top = container(stack![artwork, menu].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(crate::theme::colors(theme).border)
                    .border(iced::Border::default().rounded(iced::border::top(6)))
            });
        let labels = container(
            column![
                text(card.title).label().medium(),
                text(card.subtitle).detail().muted(),
            ]
            .spacing(spacing::XS),
        )
        .width(Fill)
        .padding([spacing::MD, spacing::LG]);
        let actions = Element::new(BoundaryActions {
            content: container(main_actions).padding([0.0, spacing::SM]).into(),
        });
        let bottom = stack![labels, actions].width(Fill);

        Card::new(column![top, bottom].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

/// Positions actions across the boundary without creating a widget overlay.
struct BoundaryActions<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for BoundaryActions<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let actions = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = actions.size();
        layout::Node::with_children(size, vec![actions.move_to((0.0, -size.height / 2.0))])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout.children().next().expect("actions layout"),
            renderer,
            operation,
        );
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().expect("actions layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("actions layout"),
            cursor,
            viewport,
            renderer,
        )
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().expect("actions layout"),
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("actions layout"),
            renderer,
            viewport,
            translation,
        )
    }
}
