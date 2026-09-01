use iced::{
    Background, Border, Theme,
    widget::{button, text},
};

use super::control::ActionState;

pub(crate) fn action(theme: &Theme, state: ActionState) -> button::Style {
    let colors = crate::theme::colors(theme);
    let background = if state.pressed {
        Some(colors.selection)
    } else if state.hovered || state.focused {
        Some(colors.hover)
    } else {
        None
    };

    button::Style {
        background: background.map(Background::Color),
        text_color: colors.text,
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}

pub(crate) fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(crate::theme::colors(theme).muted),
    }
}
