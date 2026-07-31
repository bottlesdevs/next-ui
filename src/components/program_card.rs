use iced::{
    Center, Element, Fill,
    widget::{Space, column, container, image, row},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    card::{BANNER_HEIGHT, Card, image_content},
};

const ACTION_DIAMETER: f32 = 52.0;
const PRIMARY_ACTION_DIAMETER: f32 = 68.0;

pub struct ProgramCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    settings: Option<Message>,
    play: Option<Message>,
    play_loading: bool,
}

impl<'a, Message> ProgramCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: &'a str) -> Self {
        Self {
            title,
            subtitle,
            banner: None,
            settings: None,
            play: None,
            play_loading: false,
        }
    }

    pub fn settings(mut self, settings: Message) -> Self {
        self.settings = Some(settings);
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

impl<'a, Message: Clone + 'a> From<ProgramCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ProgramCard<'a, Message>) -> Self {
        let actions = column![
            Space::new().height(BANNER_HEIGHT - PRIMARY_ACTION_DIAMETER / 2.0),
            container(
                row![
                    Space::new().width(Fill),
                    Button::icon_only("Settings", Icon::Gear)
                        .diameter(ACTION_DIAMETER)
                        .on_press_maybe(card.settings),
                    Button::icon_only("Play", Icon::Play)
                        .diameter(PRIMARY_ACTION_DIAMETER)
                        .kind(ButtonKind::Primary)
                        .on_press_maybe(card.play)
                        .loading(card.play_loading),
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
