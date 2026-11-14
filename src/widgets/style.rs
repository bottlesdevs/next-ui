use crate::Theme;
use iced::widget::text;

pub(crate) fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.colors().muted),
    }
}
