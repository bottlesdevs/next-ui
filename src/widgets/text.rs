use iced::{Font, font::Weight, widget::Text};

use super::{style, text_size};

pub trait TextExt: Sized {
    fn h3(self) -> Self;
    fn h4(self) -> Self;
    fn title(self) -> Self;
    fn subtitle(self) -> Self;
    fn body(self) -> Self;
    fn label(self) -> Self;
    fn detail(self) -> Self;
    fn medium(self) -> Self;
    fn muted(self) -> Self;
}

impl<'a> TextExt for Text<'a> {
    fn h3(self) -> Self {
        self.size(54)
    }

    fn h4(self) -> Self {
        self.size(44)
    }

    fn title(self) -> Self {
        self.size(text_size::TITLE)
    }

    fn subtitle(self) -> Self {
        self.size(text_size::SUBTITLE)
    }

    fn body(self) -> Self {
        self.size(text_size::BODY)
    }

    fn label(self) -> Self {
        self.size(text_size::LABEL)
    }

    fn detail(self) -> Self {
        self.size(text_size::DETAIL)
    }

    fn medium(self) -> Self {
        self.font(Font {
            weight: Weight::Medium,
            ..Font::DEFAULT
        })
    }

    fn muted(self) -> Self {
        self.style(style::muted_text)
    }
}
