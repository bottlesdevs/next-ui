use iced::{
    Background, Border, Element, Fill, Theme,
    widget::{Space, button, container, row},
};

use super::pressable::{Pressable, Status};

const WIDTH: f32 = 52.0;
const HEIGHT: f32 = 32.0;
const KNOB: f32 = 24.0;

pub struct Switcher<'a, Message> {
    is_on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    enabled: bool,
}

impl<'a, Message> Switcher<'a, Message> {
    pub fn new(is_on: bool) -> Self {
        Self {
            is_on,
            on_toggle: None,
            enabled: true,
        }
    }

    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn on_toggle_maybe(mut self, on_toggle: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.on_toggle = on_toggle.map(|on_toggle| Box::new(on_toggle) as _);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Switcher<'a, Message>> for Element<'a, Message> {
    fn from(switcher: Switcher<'a, Message>) -> Self {
        let active = switcher.enabled && switcher.on_toggle.is_some();
        let knob = container(Space::new())
            .width(KNOB)
            .height(KNOB)
            .style(move |theme| knob_style(theme, switcher.is_on, active));
        let content = if switcher.is_on {
            row![Space::new().width(Fill), knob]
        } else {
            row![knob, Space::new().width(Fill)]
        };
        let message = switcher
            .on_toggle
            .filter(|_| switcher.enabled)
            .map(|on_toggle| on_toggle(!switcher.is_on));

        Pressable::new(content.width(Fill))
            .width(WIDTH)
            .height(HEIGHT)
            .padding((HEIGHT - KNOB) / 2.0)
            .on_press_maybe(message)
            .style(track_style)
            .into()
    }
}

fn track_style(theme: &Theme, status: Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.weaker.color,
        )),
        border: Border::default()
            .rounded(HEIGHT / 2.0)
            .color(if status == Status::Focused {
                theme.palette().primary
            } else {
                iced::Color::TRANSPARENT
            })
            .width(if status == Status::Focused { 2 } else { 0 }),
        ..button::Style::default()
    }
}

fn knob_style(theme: &Theme, is_on: bool, enabled: bool) -> container::Style {
    let color = if !enabled {
        theme.extended_palette().secondary.weak.text
    } else if is_on {
        theme.palette().primary
    } else {
        theme.extended_palette().background.stronger.color
    };

    container::Style::default()
        .background(color)
        .border(Border::default().rounded(KNOB / 2.0))
}

#[cfg(test)]
mod tests {
    use iced::Background;

    use crate::theme;

    use super::{HEIGHT, KNOB, WIDTH, knob_style};

    #[test]
    fn switcher_uses_mockup_geometry_and_state_colors() {
        let theme = theme::theme();

        assert_eq!((WIDTH, HEIGHT, KNOB), (52.0, 32.0, 24.0));
        assert_eq!(
            knob_style(&theme, false, true).background,
            Some(Background::Color(theme::SURFACE_SELECTED))
        );
        assert_eq!(
            knob_style(&theme, true, true).background,
            Some(Background::Color(theme::ACCENT))
        );
        assert_ne!(
            knob_style(&theme, true, false).background,
            knob_style(&theme, true, true).background,
        );
    }
}
