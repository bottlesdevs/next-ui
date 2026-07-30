use iced::{
    Background, Border, Element, Fill, Theme,
    widget::{Column, button, container, text},
};

use super::style;

pub struct Popover<'a, T, Message> {
    options: &'a [T],
    selected: Option<usize>,
    embedded: bool,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a, T, Message> Popover<'a, T, Message> {
    pub fn new(options: &'a [T], on_select: impl Fn(usize) -> Message + 'a) -> Self {
        Self {
            options,
            selected: None,
            embedded: false,
            on_select: Box::new(on_select),
        }
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn embedded(mut self) -> Self {
        self.embedded = true;
        self
    }
}

impl<'a, T, Message> From<Popover<'a, T, Message>> for Element<'a, Message>
where
    T: ToString + 'a,
    Message: Clone + 'a,
{
    fn from(popover: Popover<'a, T, Message>) -> Self {
        let Popover {
            options,
            selected,
            embedded,
            on_select,
        } = popover;
        let rows = options.iter().enumerate().map(move |(index, label)| {
            let selected = selected == Some(index);

            button(text(label.to_string()).size(28))
                .padding([12, 18])
                .width(Fill)
                .style(move |theme, status| option_style(theme, status, selected))
                .on_press(on_select(index))
                .into()
        });

        let content = container(Column::with_children(rows).width(Fill))
            .padding(22)
            .width(Fill);

        if embedded {
            content.into()
        } else {
            content.style(style::surface).into()
        }
    }
}

fn option_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let selected_colors = palette.background.stronger;
    let highlighted =
        selected || matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: highlighted.then_some(Background::Color(selected_colors.color)),
        text_color: if highlighted {
            selected_colors.text
        } else {
            palette.secondary.weak.text
        },
        border: Border::default().rounded(12),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::{Background, widget::button};

    use crate::theme;

    use super::option_style;

    #[test]
    fn selected_option_is_highlighted() {
        let theme = theme::theme();

        assert_eq!(
            option_style(&theme, button::Status::Active, true).background,
            Some(Background::Color(
                theme.extended_palette().background.stronger.color
            ))
        );
    }
}
