use iced::{
    Background, Border, Shadow, Theme,
    widget::{button, container, text},
};

pub fn surface(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette();

    container::Style {
        text_color: Some(colors.secondary.base.text),
        background: Some(Background::Color(colors.secondary.base.color)),
        border: Border::default().rounded(8),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn action(theme: &Theme, status: button::Status) -> button::Style {
    let colors = theme.extended_palette();
    let colors = match status {
        button::Status::Hovered => colors.secondary.strong,
        _ => colors.secondary.base,
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(8),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn tab(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Disabled => theme.extended_palette().secondary.weak.text,
            _ => theme.palette().text,
        },
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.weak.text),
    }
}
