use iced::{
    Center, Element, Length, Theme,
    widget::{column, container, row, svg, text, text::Fragment, text::IntoFragment},
};

use crate::{
    icons::Icon,
    theme::{self, Surface},
};

use super::{spacing, text::TextExt as _, text_size};

pub use crate::theme::Tone as Kind;

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
                row![icon(kind), text(title).size(text_size::HEADING).medium(),]
                    .spacing(8)
                    .align_y(Center),
                text(body).size(text_size::SMALL),
            ]
            .spacing(spacing::SM),
        )
        .width(width)
        .height(height)
        .padding(16)
        .clip(true)
        .style(theme::surface(Surface::Banner(kind)))
        .into()
    }
}

pub(super) fn icon<'a>(kind: Kind) -> svg::Svg<'a> {
    let icon = match kind {
        Kind::Hint => Icon::Wand,
        Kind::Info => Icon::Info,
        Kind::Error => Icon::Error,
        Kind::Warning => Icon::Warning,
        Kind::Success => Icon::DoubleCheckmark,
    };

    icon.view()
        .width(text_size::HEADING)
        .height(text_size::HEADING)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(Surface::Banner(kind).colors(theme).text),
        })
}
