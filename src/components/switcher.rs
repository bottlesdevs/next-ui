use iced::{
    Background, Border, Element, Fill, Theme,
    widget::{Space, button, container, row},
};

const WIDTH: f32 = 52.0;
const HEIGHT: f32 = 32.0;
const KNOB: f32 = 24.0;

pub struct Switcher<'a, Message> {
    is_on: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message> Switcher<'a, Message> {
    pub fn new(is_on: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        Self {
            is_on,
            on_toggle: Box::new(on_toggle),
        }
    }
}

impl<'a, Message: Clone + 'a> From<Switcher<'a, Message>> for Element<'a, Message> {
    fn from(switcher: Switcher<'a, Message>) -> Self {
        let knob = container(Space::new())
            .width(KNOB)
            .height(KNOB)
            .style(move |theme| knob_style(theme, switcher.is_on));
        let content = if switcher.is_on {
            row![Space::new().width(Fill), knob]
        } else {
            row![knob, Space::new().width(Fill)]
        };

        button(content.width(Fill))
            .width(WIDTH)
            .height(HEIGHT)
            .padding((HEIGHT - KNOB) / 2.0)
            .on_press((switcher.on_toggle)(!switcher.is_on))
            .style(track_style)
            .into()
    }
}

fn track_style(theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.weaker.color,
        )),
        border: Border::default().rounded(HEIGHT / 2.0),
        ..button::Style::default()
    }
}

fn knob_style(theme: &Theme, is_on: bool) -> container::Style {
    let color = if is_on {
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
    use iced::{Background, widget::button};

    use crate::theme;

    use super::{HEIGHT, KNOB, WIDTH, knob_style, track_style};

    #[test]
    fn switcher_uses_mockup_colors() {
        let theme = theme::theme();

        assert_eq!((WIDTH, HEIGHT, KNOB), (52.0, 32.0, 24.0));
        assert_eq!(
            track_style(&theme, button::Status::Active).background,
            Some(Background::Color(theme::PANEL))
        );
        assert_eq!(
            track_style(&theme, button::Status::Hovered).background,
            Some(Background::Color(theme::PANEL))
        );
        assert_eq!(
            track_style(&theme, button::Status::Pressed).background,
            Some(Background::Color(theme::PANEL))
        );
        assert_eq!(
            knob_style(&theme, false).background,
            Some(Background::Color(theme::SURFACE_SELECTED))
        );
        assert_eq!(
            knob_style(&theme, true).background,
            Some(Background::Color(theme::ACCENT))
        );
    }
}
