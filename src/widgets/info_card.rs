use iced::{
    Center, Element, Theme,
    theme::palette::Pair,
    widget::{column, row, text, text::Fragment, text::IntoFragment},
};

use crate::{icons::Icon, theme};

use super::{card::Card, spacing, text::TextExt as _};

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
}

impl<'a> InfoCard<'a> {
    pub fn new(kind: Kind, title: impl IntoFragment<'a>, body: impl IntoFragment<'a>) -> Self {
        Self {
            kind,
            title: title.into_fragment(),
            body: body.into_fragment(),
        }
    }
}

impl<'a, Message: 'a> From<InfoCard<'a>> for Element<'a, Message> {
    fn from(card: InfoCard<'a>) -> Self {
        let InfoCard { kind, title, body } = card;

        Card::new(
            column![
                row![icon(kind), text(title).size(17).medium(),]
                    .spacing(spacing::SM)
                    .align_y(Center),
                text(body).size(14),
            ]
            .spacing(spacing::SM),
        )
        .padding(spacing::LG)
        .style(move |current_theme| theme::surface(colors(current_theme, kind)))
        .into()
    }
}

fn icon<'a, Message: 'a>(kind: Kind) -> Element<'a, Message> {
    match kind {
        Kind::Hint => Icon::Wand.view(),
        Kind::Info => Icon::Info.view(),
        Kind::Error => Icon::Error.view(),
        Kind::Warning => Icon::Warning.view(),
        Kind::Success => Icon::DoubleCheckmark.view(),
    }
}

fn colors(theme: &Theme, kind: Kind) -> Pair {
    let palette = theme.extended_palette();

    match kind {
        Kind::Hint => theme::BottlesTheme::from(theme).hint,
        Kind::Info => theme::info(),
        Kind::Error => palette.danger.base,
        Kind::Warning => palette.warning.base,
        Kind::Success => palette.success.base,
    }
}
