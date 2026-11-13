//! The toolkit's one styled text input: borderless and transparent, so it
//! reads as text inside whatever row or field hosts it.

use iced::{
    Background, Border, Color, Element, Fill, Theme,
    widget::{Id, text_input},
};

pub(crate) struct TextField<'a, Message> {
    placeholder: &'a str,
    value: &'a str,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    id: Option<Id>,
    size: Option<f32>,
    secure: bool,
    error: bool,
    muted: bool,
}

impl<'a, Message> TextField<'a, Message> {
    pub(crate) fn new(placeholder: &'a str, value: &'a str) -> Self {
        Self {
            placeholder,
            value,
            on_input: None,
            on_submit: None,
            id: None,
            size: None,
            secure: false,
            error: false,
            muted: false,
        }
    }

    pub(crate) fn on_input_maybe(
        mut self,
        on_input: Option<impl Fn(String) -> Message + 'a>,
    ) -> Self {
        self.on_input = on_input.map(|on_input| Box::new(on_input) as _);
        self
    }

    pub(crate) fn on_submit_maybe(mut self, on_submit: Option<Message>) -> Self {
        self.on_submit = on_submit;
        self
    }

    pub(crate) fn id(mut self, id: impl Into<Option<Id>>) -> Self {
        self.id = id.into();
        self
    }

    pub(crate) fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub(crate) fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Outlines the field in the danger colour.
    pub(crate) fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    /// Shows the value in the muted colour, as a row's subtitle line does.
    pub(crate) fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }
}

impl<'a, Message: Clone + 'a> From<TextField<'a, Message>> for Element<'a, Message> {
    fn from(field: TextField<'a, Message>) -> Self {
        let (error, muted) = (field.error, field.muted);
        let mut input = text_input(field.placeholder, field.value)
            .width(Fill)
            .padding(0)
            .secure(field.secure)
            .on_input_maybe(field.on_input)
            .on_submit_maybe(field.on_submit)
            .style(move |theme, _| style(theme, error, muted));

        if let Some(size) = field.size {
            input = input.size(size);
        }

        if let Some(id) = field.id {
            input = input.id(id);
        }

        input.into()
    }
}

fn style(theme: &Theme, error: bool, muted: bool) -> text_input::Style {
    let colors = crate::theme::colors(theme);

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default().color(if error {
            colors.danger
        } else {
            Color::TRANSPARENT
        }),
        icon: colors.muted,
        placeholder: colors.muted,
        value: if muted { colors.muted } else { colors.text },
        selection: colors.accent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_borders_match_the_original_visual_contract() {
        let theme = crate::theme::dark();
        let colors = crate::theme::colors(&theme);
        let error = style(&theme, true, true);
        let idle = style(&theme, false, true);

        assert_eq!(error.border.color, colors.danger);
        assert_eq!(error.border.width, 0.0);
        assert_eq!(idle.border.width, 0.0);
    }
}
