use iced::{
    Center, ContentFit, Element, Length, Theme,
    theme::palette::Pair,
    widget::{column, container, row, svg, text, text::Fragment, text::IntoFragment},
};

use crate::{icons::Icon, theme};

use super::{spacing, text::TextExt as _};

const TITLE_SIZE: f32 = 17.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Hint,
    Info,
    Error,
    Warning,
    Success,
}

pub struct InfoCard<'a> {
    kind: Kind,
    title: Fragment<'a>,
    body: Fragment<'a>,
    width: Length,
    height: Length,
}

impl<'a> InfoCard<'a> {
    pub fn new(kind: Kind, title: impl IntoFragment<'a>, body: impl IntoFragment<'a>) -> Self {
        Self {
            kind,
            title: title.into_fragment(),
            body: body.into_fragment(),
            width: Length::Shrink,
            height: Length::Shrink,
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
}

impl<'a, Message: 'a> From<InfoCard<'a>> for Element<'a, Message> {
    fn from(card: InfoCard<'a>) -> Self {
        let InfoCard {
            kind,
            title,
            body,
            width,
            height,
        } = card;

        container(
            column![
                row![icon(kind), text(title).size(TITLE_SIZE).medium(),]
                    .spacing(8)
                    .align_y(Center),
                text(body).size(14),
            ]
            .spacing(spacing::SM),
        )
        .width(width)
        .height(height)
        .padding(16)
        .clip(true)
        .style(move |theme| theme::surface(colors(theme, kind)))
        .into()
    }
}

fn icon<'a, Message: 'a>(kind: Kind) -> Element<'a, Message> {
    let icon = match kind {
        Kind::Hint => Icon::Wand,
        Kind::Info => Icon::Info,
        Kind::Error => Icon::Error,
        Kind::Warning => Icon::Warning,
        Kind::Success => Icon::DoubleCheckmark,
    };

    svg(icon.handle())
        .width(TITLE_SIZE)
        .height(TITLE_SIZE)
        .content_fit(ContentFit::Contain)
        .into()
}

fn colors(theme: &Theme, kind: Kind) -> Pair {
    let palette = theme.extended_palette();

    match kind {
        Kind::Hint => theme::hint(theme),
        Kind::Info => theme::info(),
        Kind::Error => palette.danger.base,
        Kind::Warning => palette.warning.base,
        Kind::Success => palette.success.base,
    }
}
