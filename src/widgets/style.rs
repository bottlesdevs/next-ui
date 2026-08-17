use iced::{
    Background, Border, Theme,
    theme::palette::Pair,
    widget::{button, container, text},
};

use super::pressable::Status;

pub(crate) fn surface(theme: &Theme) -> container::Style {
    surface_style(theme.extended_palette().background.weak)
}

pub(crate) fn panel(theme: &Theme) -> container::Style {
    surface_style(theme.extended_palette().background.weaker)
}

fn surface_style(colors: Pair) -> container::Style {
    container::Style::default()
        .color(colors.text)
        .background(colors.color)
        .border(Border::default().rounded(8))
}

pub(crate) fn action(theme: &Theme, status: Status) -> button::Style {
    let background = match status {
        Status::Hovered | Status::Focused => Some(crate::theme::ROW_HOVER_STRONG),
        Status::Pressed => Some(theme.extended_palette().background.stronger.color),
        _ => None,
    };

    button::Style {
        background: background.map(Background::Color),
        text_color: theme.palette().text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

pub(crate) fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
