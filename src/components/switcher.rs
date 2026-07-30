use iced::{Background, Color, Element, Theme, widget::toggler};

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

impl<'a, Message: 'a> From<Switcher<'a, Message>> for Element<'a, Message> {
    fn from(switcher: Switcher<'a, Message>) -> Self {
        toggler(switcher.is_on)
            .size(52)
            .on_toggle(switcher.on_toggle)
            .style(appearance)
            .into()
    }
}

fn appearance(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let is_on = match status {
        toggler::Status::Active { is_toggled }
        | toggler::Status::Hovered { is_toggled }
        | toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    let colors = theme.extended_palette();

    toggler::Style {
        background: Background::Color(colors.secondary.base.color),
        foreground: Background::Color(if is_on {
            colors.primary.base.color
        } else {
            colors.secondary.strong.color
        }),
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(theme.palette().text),
        border_radius: None,
        padding_ratio: 0.1,
    }
}
