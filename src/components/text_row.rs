use iced::{
    Background, Border, Center, Color, ContentFit, Element, Fill, Theme,
    widget::{Id, column, row, svg, text, text_input},
};

use crate::icons;

use super::list_row::ListRow;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Variant {
    #[default]
    One,
    Two,
    Three,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    title_size: f32,
    placeholder_size: f32,
    spacing: f32,
    icon_size: f32,
}

pub struct TextRow<'a, Message> {
    title: &'a str,
    placeholder: &'a str,
    value: &'a str,
    icon: Option<svg::Handle>,
    variant: Variant,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    id: Option<Id>,
    on_press: Option<Message>,
}

impl<'a, Message> TextRow<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: "",
            placeholder: "",
            value: "",
            icon: None,
            variant: Variant::One,
            on_input: None,
            id: None,
            on_press: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = value;
        self
    }

    pub fn icon(mut self, icon: impl Into<svg::Handle>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn variant_1(mut self) -> Self {
        self.variant = Variant::One;
        self
    }

    pub fn variant_2(mut self) -> Self {
        self.variant = Variant::Two;
        self
    }

    pub fn variant_3(mut self) -> Self {
        self.variant = Variant::Three;
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
        let layout = layout(text_row.variant);
        let placeholder_is_active = text_row.variant == Variant::Two;
        let title_is_muted = text_row.variant == Variant::Two;

        let mut placeholder = row![].spacing(12).align_y(Center);

        if let Some(icon) = text_row.icon {
            placeholder =
                placeholder.push(icon_view(icon, layout.icon_size, placeholder_is_active));
        }

        let mut input = text_input(text_row.placeholder, text_row.value)
            .width(Fill)
            .padding(0)
            .size(layout.placeholder_size)
            .style(move |theme, _| input_style(theme, placeholder_is_active))
            .on_input_maybe(text_row.on_input);

        if let Some(id) = text_row.id {
            input = input.id(id);
        }

        placeholder = placeholder.push(input);

        let labels = column![
            text(text_row.title)
                .size(layout.title_size)
                .style(move |theme: &Theme| text::Style {
                    color: Some(foreground(theme, !title_is_muted)),
                }),
            placeholder,
        ]
        .width(Fill)
        .spacing(layout.spacing);

        let row = ListRow::new(labels)
            .trailing(icon_view(icons::get("pencil"), 16.0, false))
            .height(79)
            .padding([0, 24])
            .raised(text_row.variant == Variant::Three);

        if let Some(on_press) = text_row.on_press {
            row.on_press_area(on_press)
        } else {
            row
        }
    }
}

fn layout(variant: Variant) -> Layout {
    match variant {
        Variant::One => Layout {
            title_size: 18.0,
            placeholder_size: 16.0,
            spacing: 4.0,
            icon_size: 10.0,
        },
        Variant::Two => Layout {
            title_size: 14.0,
            placeholder_size: 16.0,
            spacing: 8.0,
            icon_size: 10.0,
        },
        Variant::Three => Layout {
            title_size: 16.0,
            placeholder_size: 18.0,
            spacing: 8.0,
            icon_size: 12.0,
        },
    }
}

fn input_style(theme: &Theme, active: bool) -> text_input::Style {
    let foreground = foreground(theme, active);

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: foreground,
        placeholder: foreground,
        value: foreground,
        selection: theme.palette().primary,
    }
}

fn icon_view<'a, Message: 'a>(
    handle: svg::Handle,
    size: f32,
    active: bool,
) -> Element<'a, Message> {
    svg(handle)
        .width(size)
        .height(size)
        .content_fit(ContentFit::Contain)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(foreground(theme, active)),
        })
        .into()
}

fn foreground(theme: &Theme, active: bool) -> iced::Color {
    if active {
        theme.palette().text
    } else {
        theme.extended_palette().secondary.base.text
    }
}

#[cfg(test)]
mod tests {
    use super::{Layout, Variant, layout};

    #[test]
    fn variants_have_distinct_mockup_typography() {
        assert_eq!(
            layout(Variant::One),
            Layout {
                title_size: 18.0,
                placeholder_size: 16.0,
                spacing: 4.0,
                icon_size: 10.0,
            }
        );
        assert_ne!(layout(Variant::One), layout(Variant::Two));
        assert_ne!(layout(Variant::Two), layout(Variant::Three));

        assert_ne!(Variant::One, Variant::Three);
    }
}
