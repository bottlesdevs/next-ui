use iced::{
    Center, ContentFit, Fill, Length, Size,
    advanced::{layout, widget::Tree},
    widget::{Space, column, container, image, row, stack, text},
};

use super::{arranged::Arrangement, radius};
use crate::{Element, Theme, icons::Icon, widgets::button::ButtonKind};

use super::{button::Button, card::Card, spacing, text::TextExt as _};

const ACTION_DIAMETER: f32 = 40.0;
const PRIMARY_ACTION_DIAMETER: f32 = 48.0;
/// The menu button's target; its icon stays 16px.
const MENU_DIAMETER: f32 = 32.0;
const MENU_ICON: f32 = 16.0;

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
    subtitle: iced::advanced::text::Fragment<'a>,
    banner: Option<image::Handle>,
    menu: Option<CardAction<'a, Message>>,
    secondary: Option<CardAction<'a, Message>>,
    primary: Option<CardAction<'a, Message>>,
}

impl<'a, Message> ArtworkCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: impl iced::advanced::text::IntoFragment<'a>) -> Self {
        Self {
            title,
            subtitle: subtitle.into_fragment(),
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
                    .kind(ButtonKind::Flat)
                    .diameter(MENU_DIAMETER)
                    .icon_size(MENU_ICON),
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

        // Inset so the icon, not the larger target, sits `SM` from the corner.
        let menu = container(menu)
            .padding(spacing::SM - (MENU_DIAMETER - MENU_ICON) / 2.0)
            .width(Fill)
            .height(Fill);
        let artwork: Element<'a, Message> = match card.banner {
            Some(handle) => image(handle)
                .width(Fill)
                .height(Fill)
                .content_fit(ContentFit::Cover)
                .border_radius(radius::ROW)
                .into(),
            None => Space::new().width(Fill).height(Fill).into(),
        };
        let top = container(stack![artwork, menu].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.colors().raised)
                    .border(iced::Border::default().rounded(iced::border::top(radius::ROW)))
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
        let actions =
            BoundaryActions.arrange([container(main_actions).padding([0.0, spacing::SM]).into()]);
        let bottom = stack![labels, actions].width(Fill);

        Card::new(column![top, bottom].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

/// Positions actions across the boundary without creating a widget overlay:
/// its one child, raised by half its height.
struct BoundaryActions;

impl Arrangement for BoundaryActions {
    fn size<Message>(&self, children: &[Element<'_, Message>]) -> Size<Length> {
        children[0].as_widget().size()
    }

    fn layout<Message>(
        &self,
        children: &mut [Element<'_, Message>],
        trees: &mut [Tree],
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let actions = children[0]
            .as_widget_mut()
            .layout(&mut trees[0], renderer, limits);
        let size = actions.size();
        layout::Node::with_children(size, vec![actions.move_to((0.0, -size.height / 2.0))])
    }
}
