use iced::{
    Alignment, Background, Border, Element, Fill, Theme,
    widget::{Space, button, column, row, svg, text, text::Fragment},
};

use crate::icons::Icon;

use super::{
    control::{Control, State},
    spacing,
    text::TextExt as _,
};

pub(super) fn row_content<'a, Message: 'a>(
    title: Fragment<'a>,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
) -> iced::widget::Row<'a, Message> {
    let mut labels = column![text(title).label()].spacing(spacing::XS);

    if let Some(subtitle) = subtitle {
        labels = labels.push(text(subtitle).detail().muted());
    }

    let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        content = content.push(
            svg(icon.handle())
                .width(20)
                .height(20)
                .content_fit(iced::ContentFit::Contain),
        );
    }

    content.push(labels).push(Space::new().width(Fill))
}

pub(super) fn item<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    selected: bool,
    keyboard_highlighted: impl Fn() -> bool + 'a,
) -> Element<'a, Message> {
    Control::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press_maybe(on_press)
        .selected(selected)
        .style(move |theme, mut state| {
            state.keyboard_highlighted = keyboard_highlighted();
            row_style(theme, state)
        })
        .into()
}

pub(super) fn footer<'a, Message: Clone + 'a>(
    label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    Control::new(
        row![text(label), Icon::Arrow.rotated(std::f32::consts::PI)]
            .spacing(spacing::SM)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(spacing::MD)
    .on_press(message)
    .style(row_style)
    .into()
}

fn row_style(theme: &Theme, state: State) -> button::Style {
    let highlighted = state.actionable
        && (state.keyboard_highlighted || state.hovered || state.pressed || state.focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}
