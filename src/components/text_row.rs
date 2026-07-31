use iced::{
    Background, Border, Center, Color, ContentFit, Element, Fill, Theme,
    widget::{Id, column, row, svg, text, text_input},
};

use crate::icons::Icon;

use super::{list_row::ListRow, text::TextExt as _};

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
    enabled: bool,
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
            enabled: true,
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

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
        let mut value = row![].spacing(12).align_y(Center);

        if let Some(icon) = text_row.icon {
            value = value.push(icon_view(icon, 12.0, error));
        }

        let mut input = text_input(text_row.placeholder, text_row.value)
            .width(Fill)
            .padding(0)
            .size(16)
            .secure(text_row.secure)
            .on_input_maybe(text_row.on_input)
            .on_submit_maybe(text_row.on_submit)
            .style(move |theme, status| input_style(theme, status, error));

        if let Some(id) = text_row.id {
            input = input.id(id);
        }

        value = value.push(input);

        let mut labels = column![text(text_row.title).label(), value]
            .width(Fill)
            .spacing(4);

        if let Some(error) = text_row.error {
            labels = labels.push(text(error).detail().style(|theme: &Theme| text::Style {
                color: Some(theme.palette().danger),
            }));
        }

        let mut row = ListRow::new(labels)
            .enabled(text_row.enabled)
            .padding([18, 24]);

        if editable {
            row = row
                .trailing(icon_view(Icon::Pencil, 16.0, false))
                .focus_content_on_press();
        }

        row
    }
}

fn input_style(theme: &Theme, status: text_input::Status, error: bool) -> text_input::Style {
    let muted = theme.extended_palette().secondary.base.text;

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default().color(if error {
            theme.palette().danger
        } else {
            Color::TRANSPARENT
        }),
        icon: muted,
        placeholder: muted,
        value: if matches!(status, text_input::Status::Disabled) {
            muted
        } else {
            theme.palette().text
        },
        selection: theme.palette().primary,
    }
}

fn icon_view<'a, Message: 'a>(icon: Icon, size: f32, error: bool) -> Element<'a, Message> {
    svg(icon.handle())
        .width(size)
        .height(size)
        .content_fit(ContentFit::Contain)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(if error {
                theme.palette().danger
            } else {
                theme.extended_palette().secondary.base.text
            }),
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::TextRow;

    #[test]
    fn editing_is_opt_in() {
        let read_only = TextRow::<()>::new("Name", "Bottle");
        let editable = TextRow::new("Name", "Bottle").on_input(|_| ());

        assert!(read_only.on_input.is_none());
        assert!(editable.on_input.is_some());
    }
}
