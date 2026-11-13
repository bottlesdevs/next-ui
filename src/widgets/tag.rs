use iced::{
    Element,
    widget::{container, text, text::IntoFragment},
};

use super::text::TextExt as _;
use super::text_size;
use crate::theme::Surface;

/// A short label describing what something is, e.g. "Account" or "Library".
pub struct Tag<'a> {
    label: text::Fragment<'a>,
}

impl<'a> Tag<'a> {
    pub fn new(label: impl IntoFragment<'a>) -> Self {
        Self {
            label: label.into_fragment(),
        }
    }
}

impl<'a, Message: 'a> From<Tag<'a>> for Element<'a, Message> {
    fn from(tag: Tag<'a>) -> Self {
        container(text(tag.label).size(text_size::FOOTNOTE).medium())
            .padding([3, 9])
            .style(|theme| {
                Surface::Raised
                    .style(theme)
                    .color(crate::theme::colors(theme).accent_muted)
            })
            .into()
    }
}
