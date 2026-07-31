use iced::{
    Center, Element, Fill,
    widget::{Space, column, container, image, row},
};

use crate::icons::Icon;

use super::{
    button::Button,
    card::{BANNER_HEIGHT, Card, image_content},
};

const ACTION_DIAMETER: f32 = 52.0;
const PRIMARY_ACTION_DIAMETER: f32 = 68.0;

pub struct ProgramCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    settings: Message,
    play: Message,
}

impl<'a, Message> ProgramCard<'a, Message> {
    pub fn new(settings: Message, play: Message) -> Self {
        Self {
            title: "",
            subtitle: "",
            banner: None,
            settings,
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

impl<'a, Message: Clone + 'a> From<ProgramCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ProgramCard<'a, Message>) -> Self {
        let actions = column![
            Space::new().height(BANNER_HEIGHT - PRIMARY_ACTION_DIAMETER / 2.0),
            container(
                row![
                    Space::new().width(Fill),
                    Button::icon_only("Settings", Icon::Gear)
                        .diameter(ACTION_DIAMETER)
                        .on_press(card.settings),
                    Button::icon_only("Play", Icon::Play)
                        .diameter(PRIMARY_ACTION_DIAMETER)
                        .primary()
                        .on_press(card.play),
                ]
                .spacing(8)
                .align_y(Center)
                .width(Fill),
            )
            .padding([0, 12]),
        ]
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
