use iced::{
    Alignment,
    widget::{
        column, text,
        text::{Fragment, IntoFragment},
    },
};

use super::{spacing, text::TextExt as _, text_size};
use crate::Element;

enum Detail<'a> {
    Subtitle(Fragment<'a>),
    Status(Fragment<'a>),
}

/// A title with an optional detail line below it: the title is the same
/// size whether or not a detail follows it.
pub struct Title<'a> {
    title: &'a str,
    detail: Option<Detail<'a>>,
}

impl<'a> Title<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            detail: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        self.detail = Some(Detail::Subtitle(subtitle.into_fragment()));
        self
    }

    pub fn status(mut self, status: impl IntoFragment<'a>) -> Self {
        self.detail = Some(Detail::Status(status.into_fragment()));
        self
    }
}

impl<'a, Message: 'a> From<Title<'a>> for Element<'a, Message> {
    fn from(title: Title<'a>) -> Self {
        let mut content = column![text(title.title).size(text_size::HEADING)];

        if let Some(detail) = title.detail {
            let (label, is_status) = match detail {
                Detail::Subtitle(label) => (label, false),
                Detail::Status(label) => (label, true),
            };

            let label = text(label).size(text_size::CAPTION);
            content = content.push(if is_status {
                label.accent()
            } else {
                label.muted()
            });
        }

        content
            .align_x(Alignment::Center)
            .spacing(spacing::XS)
            .into()
    }
}
