use iced::{
    Center, Element, Fill, Theme,
    widget::{Id, column, row, svg, text},
};

use crate::icons::Icon;

use super::{
    list_row::{self, ListRow},
    spacing,
    text::TextExt as _,
    text_field::TextField,
};

pub struct TextRow<'a, Message> {
    title: &'a str,
    value: &'a str,
    placeholder: &'a str,
    icon: Option<Icon>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    id: Option<Id>,
    secure: bool,
    error: Option<&'a str>,
}

impl<'a, Message> TextRow<'a, Message> {
    pub fn new(title: &'a str, value: &'a str) -> Self {
        Self {
            title,
            value,
            placeholder: "",
            icon: None,
            on_input: None,
            on_submit: None,
            id: None,
            secure: false,
            error: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn on_submit(mut self, on_submit: Message) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn error(mut self, error: impl Into<Option<&'a str>>) -> Self {
        self.error = error.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<TextRow<'a, Message>> for Element<'a, Message> {
    fn from(text_row: TextRow<'a, Message>) -> Self {
        ListRow::from(text_row).into()
    }
}

impl<'a, Message: Clone + 'a> From<TextRow<'a, Message>> for ListRow<'a, Message> {
    fn from(text_row: TextRow<'a, Message>) -> Self {
        let error = text_row.error.is_some();
        let editable = text_row.on_input.is_some();
        let mut value = row![].spacing(spacing::SM).align_y(Center);

        if let Some(icon) = text_row.icon {
            value = value.push(icon_view(icon, list_row::BODY_SIZE, error));
        }

        value = value.push(
            TextField::new(text_row.placeholder, text_row.value)
                .size(list_row::BODY_SIZE)
                .secure(text_row.secure)
                .on_input_maybe(text_row.on_input)
                .on_submit_maybe(text_row.on_submit)
                .id(text_row.id)
                .error(error)
                .muted(true),
        );

        let mut labels = column![text(text_row.title).label().medium(), value]
            .width(Fill)
            .spacing(spacing::XS);

        if let Some(error) = text_row.error {
            labels = labels.push(
                text(error)
                    .size(list_row::BODY_SIZE)
                    .style(|theme: &Theme| text::Style {
                        color: Some(crate::theme::colors(theme).danger),
                    }),
            );
        }

        let mut row = ListRow::new(labels);

        if editable {
            row = row.trailing(icon_view(Icon::Pencil, list_row::BODY_SIZE, false));
        }

        row
    }
}

fn icon_view<'a, Message: 'a>(icon: Icon, size: f32, error: bool) -> Element<'a, Message> {
    icon.view()
        .width(size)
        .height(size)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if error {
                crate::theme::colors(theme).danger
            } else {
                crate::theme::colors(theme).muted
            }),
        })
        .into()
}
