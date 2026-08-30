use iced::{
    ContentFit, Element, Fill, Length, Padding, Theme,
    widget::{Space, column, container, image, stack, text},
};

use super::{
    spacing,
    surface::{Kind as SurfaceKind, Surface},
    text::TextExt as _,
};

pub(crate) const BANNER_HEIGHT: f32 = 132.0;

pub struct Card<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    padding: Padding,
}

impl<'a, Message> Card<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        Surface::new(
            SurfaceKind::Card,
            container(card.content)
                .padding(card.padding)
                .width(card.width)
                .height(card.height)
                .clip(true),
        )
        .into()
    }
}

pub(crate) fn labels<'a, Message: 'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(title).label().medium(),
        text(subtitle).detail().muted(),
    ]
    .spacing(spacing::XS)
    .into()
}

pub(crate) fn image_content<'a, Message: 'a>(
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
                    .background(theme.extended_palette().background.neutral.color)
            })
            .into(),
    };

    stack![
        column![
            banner,
            container(labels(title, subtitle)).padding([spacing::MD, spacing::LG]),
        ]
        .width(Fill),
        actions,
    ]
    .width(Fill)
    .into()
}
