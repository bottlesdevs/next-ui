use iced::{
    Element, Font,
    font::Weight,
    widget::{Text, text},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    H1,
    H2,
    H3,
    H4,
    H5,
}

pub struct Heading<'a> {
    text: Text<'a>,
}

impl<'a> Heading<'a> {
    pub fn new(level: Level, content: impl text::IntoFragment<'a>) -> Self {
        let size = match level {
            Level::H1 => 72,
            Level::H2 => 64,
            Level::H3 => 54,
            Level::H4 => 44,
            Level::H5 => 40,
        };

        Self {
            text: text(content).size(size).font(Font {
                weight: Weight::Semibold,
                ..Font::DEFAULT
            }),
        }
    }
}

impl<'a, Message: 'a> From<Heading<'a>> for Element<'a, Message> {
    fn from(heading: Heading<'a>) -> Self {
        heading.text.into()
    }
}
