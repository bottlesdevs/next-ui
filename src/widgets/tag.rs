use iced::{
    Border, Element,
    widget::{container, text, text::IntoFragment},
};

use super::text::TextExt as _;
use super::{radius, text_size};

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
                let colors = crate::theme::colors(theme);

                container::Style::default()
                    .background(colors.raised)
                    .color(colors.accent_muted)
                    .border(Border::default().rounded(radius::ROW))
            })
            .into()
    }
}
