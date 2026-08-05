use iced::{
    Element, Fill,
    widget::{Space, column, container, image, row, stack},
};

use crate::{components::button::ButtonKind, icons::Icon};

use super::{
    button::Button,
    card::{BANNER_HEIGHT, Card, image_content},
    spacing,
};

const ACTION_DIAMETER: f32 = 52.0;

pub struct ArtworkCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    menu: Option<Message>,
    play: Option<Message>,
    play_loading: bool,
}

impl<'a, Message> ArtworkCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: &'a str) -> Self {
        Self {
            title,
            subtitle,
            banner: None,
            menu: None,
            play: None,
            play_loading: false,
        }
    }

    pub fn menu(mut self, menu: Message) -> Self {
        self.menu = Some(menu);
        self
    }

    pub fn play(mut self, play: Message) -> Self {
        self.play = Some(play);
        self
    }

    pub fn play_loading(mut self, loading: bool) -> Self {
        self.play_loading = loading;
        self
    }

    pub fn banner(mut self, banner: impl Into<image::Handle>) -> Self {
        self.banner = Some(banner.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<ArtworkCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ArtworkCard<'a, Message>) -> Self {
        let actions = stack![
            container(
                row![
                    Space::new().width(Fill),
                    Button::icon_only("More actions", Icon::EllipsisVertical)
                        .kind(ButtonKind::Transparent)
                        .diameter(32.0)
                        .on_press_maybe(card.menu)
                ]
                .width(Fill),
            )
            .padding(spacing::SM)
            .width(Fill)
            .height(BANNER_HEIGHT),
            column![
                Space::new().height(BANNER_HEIGHT - ACTION_DIAMETER / 2.0),
                container(
                    row![
                        Space::new().width(Fill),
                        Button::icon_only("Play", Icon::Play)
                            .diameter(ACTION_DIAMETER)
                            .on_press_maybe(card.play)
                            .loading(card.play_loading)
                    ]
                    .width(Fill),
                )
                .padding([0.0, spacing::SM])
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
        .into()
    }
}
