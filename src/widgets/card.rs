use iced::{
    ContentFit, Element, Fill, Padding, Theme,
    widget::{Space, column, container, image, stack, text},
};

use super::{
    spacing,
    surface::{Kind as SurfaceKind, Surface},
    text::TextExt as _,
};

pub(crate) const BANNER_HEIGHT: f32 = 180.0;

pub struct Card<'a, Message> {
    content: Element<'a, Message>,
    padding: Padding,
    style: Option<container::StyleFn<'a, Theme>>,
}

impl<'a, Message> Card<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            padding: Padding::ZERO,
            style: None,
        }
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub(crate) fn style(mut self, style: impl Fn(&Theme) -> container::Style + 'a) -> Self {
        self.style = Some(Box::new(style));
        self
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        let content = container(card.content)
            .padding(card.padding)
            .width(Fill)
            .clip(true);

        match card.style {
            Some(style) => content.class(style).into(),
            None => Surface::new(SurfaceKind::Card, content).into(),
        }
    }
}

pub(crate) fn labels<'a, Message: 'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![text(title).title(), text(subtitle).subtitle().muted(),]
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
                    .background(theme.extended_palette().background.strong.color)
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
