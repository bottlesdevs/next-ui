use iced::{
    Alignment, Background, Border, Element, Fill, Theme,
    widget::{Space, button, column, row, text, text::Fragment},
};

use crate::icons::Icon;

use super::{
    focusable_action::{ActionState, FocusableAction},
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
        content = content.push(icon.view().width(20).height(20));
    }

    content.push(labels).push(Space::new().width(Fill))
}

pub(super) fn item<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
    selected: bool,
) -> FocusableAction<'a, Message> {
    FocusableAction::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press_maybe(on_press)
        .selected(selected)
        .style(row_style)
}

pub(super) fn footer<'a, Message: Clone + 'a>(
    label: &'a str,
    message: Message,
) -> FocusableAction<'a, Message> {
    FocusableAction::new(
        row![
            text(label),
            Icon::Arrow.view().rotation(std::f32::consts::PI)
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(spacing::MD)
    .on_press_maybe(Some(message))
    .style(|theme, state| button::Style {
        background: Some(Background::Color(crate::theme::colors(theme).selection)),
        border: Border::default().rounded(iced::border::bottom(6)),
        ..row_style(theme, state)
    })
}

pub(super) fn row_style(theme: &Theme, state: ActionState) -> button::Style {
    let colors = crate::theme::colors(theme);
    let highlighted = state.enabled && (state.hovered || state.pressed || state.focused);

    button::Style {
        background: highlighted.then_some(Background::Color(colors.selection)),
        text_color: if highlighted {
            colors.text
        } else {
            colors.muted
        },
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}
