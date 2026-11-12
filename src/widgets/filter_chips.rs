use iced::{
    Background, Border, Element,
    widget::{Row, text},
};

use super::focusable_action::FocusableAction;

/// A row of pill toggles where exactly one option is selected.
pub struct FilterChips<'a, T, Message> {
    options: Vec<(T, &'a str)>,
    selected: T,
    on_select: Option<Box<dyn Fn(T) -> Message + 'a>>,
}

impl<'a, T: Copy + PartialEq + 'a, Message: 'a> FilterChips<'a, T, Message> {
    /// Creates chips for `options`, labelled in order, with `selected` active.
    pub fn new(options: impl IntoIterator<Item = (T, &'a str)>, selected: T) -> Self {
        Self {
            options: options.into_iter().collect(),
            selected,
            on_select: None,
        }
    }

    pub fn on_select(mut self, on_select: impl Fn(T) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }
}

impl<'a, T: Copy + PartialEq + 'a, Message: Clone + 'a> From<FilterChips<'a, T, Message>>
    for Element<'a, Message>
{
    fn from(chips: FilterChips<'a, T, Message>) -> Self {
        let FilterChips {
            options,
            selected,
            on_select,
        } = chips;

        Row::with_children(options.into_iter().map(|(value, label)| {
            let active = value == selected;

            FocusableAction::new(text(label).size(16))
                .on_press_maybe(on_select.as_ref().map(|on_select| on_select(value)))
                .padding([6, 14])
                .style(move |theme, state| {
                    let colors = crate::theme::colors(theme);
                    let background = if active {
                        Some(colors.border)
                    } else if state.hovered || state.focused {
                        Some(colors.surface)
                    } else {
                        None
                    };

                    iced::widget::button::Style {
                        background: background.map(Background::Color),
                        text_color: if active { colors.text } else { colors.muted },
                        border: Border::default().rounded(999),
                        ..iced::widget::button::Style::default()
                    }
                })
                .into()
        }))
        .spacing(4)
        .into()
    }
}
