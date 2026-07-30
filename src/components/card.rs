use iced::{
    ContentFit, Element, Fill, Theme,
    widget::{Space, button, column, container, image, row, stack, text},
};

use crate::icons;

use super::{button::Button, style};

const ARTWORK_HEIGHT: f32 = 180.0;

pub struct TextCard<'a> {
    title: &'a str,
    subtitle: &'a str,
    body: &'a str,
}

impl<'a> TextCard<'a> {
    pub fn new(title: &'a str, subtitle: &'a str, body: &'a str) -> Self {
        Self {
            title,
            subtitle,
            body,
        }
    }
}

impl<'a, Message: 'a> From<TextCard<'a>> for Element<'a, Message> {
    fn from(card: TextCard<'a>) -> Self {
        container(
            column![
                labels(card.title, card.subtitle),
                text(card.body).size(20).style(style::muted_text)
            ]
            .spacing(18),
        )
        .padding(24)
        .width(Fill)
        .style(style::surface)
        .into()
    }
}

pub struct ArtworkCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    artwork: Option<Element<'a, Message>>,
    menu: Message,
    play: Message,
}

impl<'a, Message> ArtworkCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: &'a str, menu: Message, play: Message) -> Self {
        Self {
            title,
            subtitle,
            artwork: None,
            menu,
            play,
        }
    }

    pub fn artwork(mut self, artwork: impl Into<Element<'a, Message>>) -> Self {
        self.artwork = Some(artwork.into());
        self
    }

    pub fn image(self, handle: impl Into<image::Handle>) -> Self {
        self.artwork(
            image(handle)
                .width(Fill)
                .height(ARTWORK_HEIGHT)
                .content_fit(ContentFit::Cover),
        )
    }
}

impl<'a, Message: Clone + 'a> From<ArtworkCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ArtworkCard<'a, Message>) -> Self {
        let actions = column![
            row![
                Space::new().width(Fill),
                button("⋮").padding(6).style(style::tab).on_press(card.menu)
            ],
            Space::new().height(Fill),
            row![
                Space::new().width(Fill),
                Button::new("Play")
                    .icon(icons::play())
                    .circular()
                    .on_press(card.play)
            ],
        ]
        .height(Fill);

        card_with_artwork(card.title, card.subtitle, card.artwork, actions.into())
    }
}

pub struct ProgramCard<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    artwork: Option<Element<'a, Message>>,
    settings: Message,
    play: Message,
}

impl<'a, Message> ProgramCard<'a, Message> {
    pub fn new(title: &'a str, subtitle: &'a str, settings: Message, play: Message) -> Self {
        Self {
            title,
            subtitle,
            artwork: None,
            settings,
            play,
        }
    }

    pub fn artwork(mut self, artwork: impl Into<Element<'a, Message>>) -> Self {
        self.artwork = Some(artwork.into());
        self
    }

    pub fn image(self, handle: impl Into<image::Handle>) -> Self {
        self.artwork(
            image(handle)
                .width(Fill)
                .height(ARTWORK_HEIGHT)
                .content_fit(ContentFit::Cover),
        )
    }
}

impl<'a, Message: Clone + 'a> From<ProgramCard<'a, Message>> for Element<'a, Message> {
    fn from(card: ProgramCard<'a, Message>) -> Self {
        let actions = column![
            Space::new().height(Fill),
            row![
                Space::new().width(Fill),
                Button::new("Settings")
                    .icon(icons::settings())
                    .circular()
                    .on_press(card.settings),
                Button::new("Play")
                    .icon(icons::play())
                    .circular()
                    .diameter(68.0)
                    .primary()
                    .on_press(card.play),
            ]
            .spacing(8),
        ]
        .height(Fill);

        card_with_artwork(card.title, card.subtitle, card.artwork, actions.into())
    }
}

fn card_with_artwork<'a, Message: Clone + 'a>(
    title: &'a str,
    subtitle: &'a str,
    artwork: Option<Element<'a, Message>>,
    actions: Element<'a, Message>,
) -> Element<'a, Message> {
    let artwork = artwork.unwrap_or_else(|| {
        container(Space::new())
            .width(Fill)
            .height(ARTWORK_HEIGHT)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.extended_palette().background.strong.color)
            })
            .into()
    });

    container(
        column![
            stack![
                container(artwork).width(Fill).height(ARTWORK_HEIGHT),
                container(actions)
                    .padding(12)
                    .width(Fill)
                    .height(ARTWORK_HEIGHT),
            ]
            .width(Fill)
            .height(ARTWORK_HEIGHT),
            container(labels(title, subtitle)).padding([18, 24]),
        ]
        .width(Fill),
    )
    .width(Fill)
    .style(style::surface)
    .into()
}

fn labels<'a, Message: 'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(title).size(28).style(text::base),
        text(subtitle).size(22).style(style::muted_text),
    ]
    .spacing(4)
    .into()
}
