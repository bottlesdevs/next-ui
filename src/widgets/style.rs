use iced::{
    Background, Border, Theme,
    widget::{button, text},
};

use super::control::State;

pub(crate) fn action(theme: &Theme, state: State) -> button::Style {
    let background = if state.pressed {
        Some(theme.extended_palette().background.stronger.color)
    } else if state.hovered || state.focused {
        Some(theme.extended_palette().background.strong.color)
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
