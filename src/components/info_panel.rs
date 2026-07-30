use iced::{
    Background, Border, Center, Element, Fill, Theme,
    theme::palette::Pair,
    widget::{column, container, row, text},
};

use crate::{icons, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Hint,
    Info,
    Error,
    Warning,
    Success,
}

pub struct InfoPanel<'a> {
    kind: Kind,
    title: &'a str,
    body: &'a str,
}

impl<'a> InfoPanel<'a> {
    pub fn new(kind: Kind, title: &'a str, body: &'a str) -> Self {
        Self { kind, title, body }
    }
}

impl<'a, Message: 'a> From<InfoPanel<'a>> for Element<'a, Message> {
    fn from(panel: InfoPanel<'a>) -> Self {
        let InfoPanel { kind, title, body } = panel;

        container(
            column![
                row![icon(kind), text(title).size(28),]
                    .spacing(14)
                    .align_y(Center),
                text(body).size(20),
            ]
            .spacing(18),
        )
        .padding(28)
        .width(Fill)
        .style(move |theme| {
            let colors = colors(theme, kind);

            container::Style {
                text_color: Some(colors.text),
                background: Some(Background::Color(colors.color)),
                border: Border::default().rounded(8),
                ..container::Style::default()
            }
        })
        .into()
    }
}

fn icon<'a, Message: 'a>(kind: Kind) -> Element<'a, Message> {
    match kind {
        Kind::Hint => icons::view("wand"),
        Kind::Info => icons::view("info"),
        Kind::Error => icons::view("error"),
        Kind::Warning => icons::view("warning"),
        Kind::Success => icons::view("double_checkmark"),
    }
}

fn colors(theme: &Theme, kind: Kind) -> Pair {
    let palette = theme.extended_palette();

    match kind {
        Kind::Hint => theme::hint(),
        Kind::Info => theme::info(),
        Kind::Error => palette.danger.base,
        Kind::Warning => palette.warning.base,
        Kind::Success => palette.success.base,
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, icon};
    use iced::Element;

    #[test]
    fn embedded_icons_exist() {
        for kind in [
            Kind::Hint,
            Kind::Info,
            Kind::Error,
            Kind::Warning,
            Kind::Success,
        ] {
            let _: Element<'_, ()> = icon(kind);
        }
    }
}
