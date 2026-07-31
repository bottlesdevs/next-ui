use iced::{
    Element, Fill,
    widget::{Space, button, column, container, image, row, stack},
};

use crate::icons::Icon;

use super::{
    button::Button,
    card::{BANNER_HEIGHT, Card, image_content},
    style,
};

const ACTION_DIAMETER: f32 = 52.0;

pub struct ArtworkCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    menu: Message,
    play: Message,
}

impl<'a, Message> ArtworkCard<'a, Message> {
    pub fn new(menu: Message, play: Message) -> Self {
        Self {
            title: "",
            subtitle: "",
            banner: None,
            menu,
            play,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
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
                    button(Icon::EllipsisVertical.view())
                        .padding(6)
                        .style(style::tab)
                        .on_press(card.menu)
                ]
                .width(Fill),
            )
            .padding(12)
            .width(Fill)
            .height(BANNER_HEIGHT),
            column![
                Space::new().height(BANNER_HEIGHT - ACTION_DIAMETER / 2.0),
                container(
                    row![
                        Space::new().width(Fill),
                        Button::icon_only("Play", Icon::Play)
                            .diameter(ACTION_DIAMETER)
                            .on_press(card.play)
                    ]
                    .width(Fill),
                )
                .padding([0, 12])
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
