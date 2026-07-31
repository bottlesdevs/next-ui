use iced::widget::Text;

use super::style;

pub trait TextExt: Sized {
    fn h1(self) -> Self;
    fn h2(self) -> Self;
    fn h3(self) -> Self;
    fn h4(self) -> Self;
    fn h5(self) -> Self;
    fn title(self) -> Self;
    fn subtitle(self) -> Self;
    fn supporting(self) -> Self;
    fn body(self) -> Self;
    fn label(self) -> Self;
    fn detail(self) -> Self;
    fn caption(self) -> Self;
    fn muted(self) -> Self;
}

impl<'a> TextExt for Text<'a> {
    fn h1(self) -> Self {
        self.size(72)
    }

    fn h2(self) -> Self {
        self.size(64)
    }

    fn h3(self) -> Self {
        self.size(54)
    }

    fn h4(self) -> Self {
        self.size(44)
    }

    fn h5(self) -> Self {
        self.size(40)
    }

    fn title(self) -> Self {
        self.size(28)
    }

    fn subtitle(self) -> Self {
        self.size(22)
    }

    fn supporting(self) -> Self {
        self.size(24)
    }

    fn body(self) -> Self {
        self.size(20)
    }

    fn label(self) -> Self {
        self.size(18)
    }

    fn detail(self) -> Self {
        self.size(16)
    }

    fn caption(self) -> Self {
        self.size(14)
    }

    fn muted(self) -> Self {
        self.style(style::muted_text)
    }
}
