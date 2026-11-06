use iced::{Element, Length, Padding, widget::container};

pub struct Card<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    padding: Padding,
}

impl<'a, Message> Card<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            padding: Padding::ZERO,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        container(card.content)
            .padding(card.padding)
            .width(card.width)
            .height(card.height)
            .clip(true)
            .style(crate::theme::card)
            .into()
    }
}
