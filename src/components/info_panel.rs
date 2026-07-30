use iced::{
    Background, Border, Element, Fill, Theme,
    theme::palette::Pair,
    widget::{column, container, row, text},
};

use crate::theme;

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
                row![text(icon(kind)).size(30), text(title).size(28),].spacing(14),
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

fn icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Hint => "✦",
        Kind::Info => "ⓘ",
        Kind::Error => "✖",
        Kind::Warning => "▲",
        Kind::Success => "✓",
    }
}

fn colors(theme: &Theme, kind: Kind) -> Pair {
    let palette = theme.extended_palette();

    match kind {
        Kind::Hint => palette.secondary.base,
        Kind::Info => Pair {
            color: theme::INFO_DARK,
            text: theme.palette().text,
        },
        Kind::Error => palette.danger.base,
        Kind::Warning => palette.warning.base,
        Kind::Success => palette.success.base,
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, icon};

    #[test]
    fn error_has_error_icon() {
        assert_eq!(icon(Kind::Error), "✖");
    }
}
