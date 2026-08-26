use iced::{
    Background, Border, Theme,
    theme::palette::Pair,
    widget::{button, container, text},
};

use super::control::State;

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
        .border(Border::default().rounded(6))
}

pub(crate) fn action(theme: &Theme, state: State) -> button::Style {
    let background = if state.pressed {
        Some(theme.extended_palette().background.stronger.color)
    } else if state.hovered || state.focused {
        Some(crate::theme::BottlesTheme::from(theme).row_hover_strong)
    } else {
        None
    };

    button::Style {
        background: background.map(Background::Color),
        text_color: theme.palette().text,
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}

pub(crate) fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
