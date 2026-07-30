use iced::{
    Center, ContentFit, Element, Fill, Theme,
    widget::{Space, button, column, container, image, row, stack, text},
};

use crate::icons;

use super::{button::Button, style};

const BANNER_HEIGHT: f32 = 180.0;
const ACTION_DIAMETER: f32 = 52.0;
const PRIMARY_ACTION_DIAMETER: f32 = 68.0;

enum Variant<Message> {
    Text,
    Artwork { menu: Message, play: Message },
    Program { settings: Message, play: Message },
}

pub struct Card<'a, Message> {
    title: &'a str,
    subtitle: &'a str,
    description: Option<&'a str>,
    banner: Option<image::Handle>,
    variant: Variant<Message>,
}

impl<Message> Card<'_, Message> {
    pub fn new() -> Self {
        Self {
            title: "",
            subtitle: "",
            description: None,
            banner: None,
            variant: Variant::Text,
        }
    }
}

impl<'a, Message> Card<'a, Message> {
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn banner(mut self, banner: impl Into<image::Handle>) -> Self {
        self.banner = Some(banner.into());
        self
    }

    pub fn text(mut self) -> Self {
        self.variant = Variant::Text;
        self
    }

    pub fn artwork(mut self, menu: Message, play: Message) -> Self {
        self.variant = Variant::Artwork { menu, play };
        self
    }

    pub fn program(mut self, settings: Message, play: Message) -> Self {
        self.variant = Variant::Program { settings, play };
        self
    }
}

impl<Message> Default for Card<'_, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        match card.variant {
            Variant::Text => text_card(card.title, card.subtitle, card.description),
            Variant::Artwork { menu, play } => {
                let actions = stack![
                    container(
                        row![
                            Space::new().width(Fill),
                            button(icons::view("ellipsis_vertical"))
                                .padding(6)
                                .style(style::tab)
                                .on_press(menu)
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
                                Button::new("Play")
                                    .icon(icons::get("play"))
                                    .circular()
                                    .diameter(ACTION_DIAMETER)
                                    .on_press(play)
                            ]
                            .width(Fill),
                        )
                        .padding([0, 12])
                    ],
                ]
                .width(Fill)
                .height(Fill);

                image_card(card.title, card.subtitle, card.banner, actions.into())
            }
            Variant::Program { settings, play } => {
                let actions = column![
                    Space::new().height(BANNER_HEIGHT - PRIMARY_ACTION_DIAMETER / 2.0),
                    container(
                        row![
                            Space::new().width(Fill),
                            Button::new("Settings")
                                .icon(icons::get("gear"))
                                .circular()
                                .diameter(ACTION_DIAMETER)
                                .on_press(settings),
                            Button::new("Play")
                                .icon(icons::get("play"))
                                .circular()
                                .diameter(PRIMARY_ACTION_DIAMETER)
                                .primary()
                                .on_press(play),
                        ]
                        .spacing(8)
                        .align_y(Center)
                        .width(Fill),
                    )
                    .padding([0, 12]),
                ]
                .height(Fill);

                image_card(card.title, card.subtitle, card.banner, actions.into())
            }
        }
    }
}

fn text_card<'a, Message: 'a>(
    title: &'a str,
    subtitle: &'a str,
    description: Option<&'a str>,
) -> Element<'a, Message> {
    let mut content = column![labels(title, subtitle)].spacing(18);

    if let Some(description) = description {
        content = content.push(text(description).size(20).style(style::muted_text));
    }

    container(content)
        .padding(24)
        .width(Fill)
        .style(style::bordered_surface)
        .into()
}

fn image_card<'a, Message: Clone + 'a>(
    title: &'a str,
    subtitle: &'a str,
    banner: Option<image::Handle>,
    actions: Element<'a, Message>,
) -> Element<'a, Message> {
    let banner: Element<'a, Message> = match banner {
        Some(handle) => image(handle)
            .width(Fill)
            .height(BANNER_HEIGHT)
            .content_fit(ContentFit::Cover)
            .into(),
        None => container(Space::new())
            .width(Fill)
            .height(BANNER_HEIGHT)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.extended_palette().background.strong.color)
            })
            .into(),
    };

    container(
        stack![
            column![
                container(banner).width(Fill).height(BANNER_HEIGHT),
                container(labels(title, subtitle)).padding([18, 24]),
            ]
            .width(Fill),
            actions,
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

#[cfg(test)]
mod tests {
    use super::{Card, Variant};

    #[test]
    fn selects_each_variant() {
        assert!(matches!(Card::<()>::new().text().variant, Variant::Text));
        assert!(matches!(
            Card::new().artwork((), ()).variant,
            Variant::Artwork { .. }
        ));
        assert!(matches!(
            Card::new().program((), ()).variant,
            Variant::Program { .. }
        ));
    }
}
