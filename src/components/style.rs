use iced::{
    Background, Border, Theme,
    theme::palette::Pair,
    widget::{button, container, text},
};

pub fn surface(theme: &Theme) -> container::Style {
    surface_style(theme.extended_palette().background.weak)
}

pub fn bordered_surface(theme: &Theme) -> container::Style {
    surface_style(theme.extended_palette().background.neutral)
}

pub fn panel(theme: &Theme) -> container::Style {
    surface_style(theme.extended_palette().background.weaker)
}

fn surface_style(colors: Pair) -> container::Style {
    container::Style::default()
        .color(colors.text)
        .background(colors.color)
        .border(Border::default().rounded(8))
}

pub fn action(theme: &Theme, status: button::Status) -> button::Style {
    let colors = theme.extended_palette();
    let colors = match status {
        button::Status::Hovered => colors.background.strong,
        button::Status::Pressed => colors.background.stronger,
        _ => colors.background.weak,
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

pub fn tab(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        text_color: match status {
            button::Status::Disabled => theme.extended_palette().secondary.weak.text,
            _ => theme.palette().text,
        },
        ..button::Style::default()
    }
}

pub fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
