use iced::{Theme, widget::text};

pub(crate) fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(crate::theme::colors(theme).muted),
    }
}
