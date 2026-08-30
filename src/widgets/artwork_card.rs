use iced::{
    Center, Element, Fill,
    widget::{Space, column, container, image, row, stack},
};

use crate::{icons::Icon, widgets::button::ButtonKind};

use super::{
    button::Button,
    card::{BANNER_HEIGHT, Card, image_content},
    spacing,
};

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
            menu = menu.push(action.button().kind(ButtonKind::Transparent).diameter(32.0));
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

        let actions = stack![
            container(menu)
                .padding(spacing::SM)
                .width(Fill)
                .height(BANNER_HEIGHT),
            column![
                Space::new().height(BANNER_HEIGHT - PRIMARY_ACTION_DIAMETER / 2.0),
                container(main_actions).padding([0.0, spacing::SM])
            ],
        ]
        .width(Fill)
        .height(Fill);

        Card::new(image_content(
            card.title,
            card.subtitle,
            card.banner,
            actions.into(),
        ))
        .width(Fill)
        .into()
    }
}
