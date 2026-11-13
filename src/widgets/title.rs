use iced::{
    Alignment, Element, Theme,
    widget::{
        column, text,
        text::{Fragment, IntoFragment},
    },
};

use super::{spacing, text_size};

enum Detail<'a> {
    Subtitle(Fragment<'a>),
    Status(Fragment<'a>),
}

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
        let (title_size, detail_size) = typography(title.detail.as_ref());
        let mut content = column![text(title.title).size(title_size).style(text::base)];

        if let Some(detail) = title.detail {
            let (label, is_status) = match detail {
                Detail::Subtitle(label) => (label, false),
                Detail::Status(label) => (label, true),
            };

            content = content.push(text(label).size(detail_size).style(move |theme: &Theme| {
                text::Style {
                    color: Some(if is_status {
                        crate::theme::colors(theme).accent_muted
                    } else {
                        crate::theme::colors(theme).muted
                    }),
                }
            }));
        }

        content
            .align_x(Alignment::Center)
            .spacing(spacing::XS)
            .into()
    }
}

fn typography(detail: Option<&Detail<'_>>) -> (f32, f32) {
    match detail {
        Some(Detail::Subtitle(_)) => (text_size::HEADING, text_size::CAPTION),
        Some(Detail::Status(_)) => (text_size::HEADLINE, text_size::BODY),
        None => (text_size::DISPLAY, text_size::HEADLINE),
    }
}
