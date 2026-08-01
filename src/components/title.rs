use iced::{
    Element, Theme,
    widget::{column, text},
};

use super::spacing;

enum Detail<'a> {
    Subtitle(&'a str),
    Status(&'a str),
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

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.detail = Some(Detail::Subtitle(subtitle));
        self
    }

    pub fn status(mut self, status: &'a str) -> Self {
        self.detail = Some(Detail::Status(status));
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
                        theme.extended_palette().primary.strong.color
                    } else {
                        theme.extended_palette().secondary.base.text
                    }),
                }
            }));
        }

        content.spacing(spacing::SM).into()
    }
}

fn typography(detail: Option<&Detail<'_>>) -> (f32, f32) {
    match detail {
        Some(Detail::Status(_)) => (48.0, 40.0),
        _ => (32.0, 24.0),
    }
}
