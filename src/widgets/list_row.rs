use iced::{
    Alignment, Background, Border, Element, Fill, Padding, Theme,
    border::Radius,
    widget::{Row as IcedRow, Space, button, column, container, text, text::IntoFragment},
};

use super::{
    focusable_action::{ActionState, FocusableAction},
    spacing,
    text::TextExt as _,
};

#[doc(hidden)]
pub const BODY_SIZE: f32 = 16.0;
/// The corner radius of a row standing on its own.
const RADIUS: f32 = 6.0;

pub(crate) const STANDARD_PADDING: Padding = Padding {
    top: spacing::MD,
    right: spacing::MD,
    bottom: spacing::MD,
    left: spacing::LG,
};

/// A non-clickable row that may contain interactive controls.
pub struct ListRow<'a, Message> {
    body: Element<'a, Message>,
    leading: Vec<Element<'a, Message>>,
    trailing: Vec<Element<'a, Message>>,
    padding: Padding,
    corners: Radius,
}

#[doc(hidden)]
pub fn labels<'a, Message: 'a>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
) -> Element<'a, Message> {
    column![
        text(title).label().medium(),
        text(description).size(BODY_SIZE).muted(),
    ]
    .spacing(spacing::XS)
    .into()
}

impl<'a, Message: 'a> ListRow<'a, Message> {
    pub fn new(body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            body: body.into(),
            leading: Vec::new(),
            trailing: Vec::new(),
            padding: STANDARD_PADDING,
            corners: Radius::new(RADIUS),
        }
    }

    pub fn leading(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.leading.push(control.into());
        self
    }

    pub fn trailing(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.trailing.push(control.into());
        self
    }

    pub fn prepend_trailing(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.trailing.insert(0, control.into());
        self
    }

    pub(crate) fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = body.into();
        self
    }

    /// Rounds the row's corners as a [`BoxedGroup`] places it.
    ///
    /// [`BoxedGroup`]: super::boxed_group::BoxedGroup
    pub(crate) fn corners(mut self, corners: Radius) -> Self {
        self.corners = corners;
        self
    }

    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub(crate) fn into_content(self) -> Element<'a, Message> {
        self.lower(false)
    }

    pub(crate) fn into_disclosure_content(self) -> Element<'a, Message> {
        self.lower(true)
    }

    fn lower(self, disclosure: bool) -> Element<'a, Message> {
        let mut trailing = self.trailing;

        if disclosure {
            trailing.push(Space::new().width(20).height(20).into());
        }

        let row = IcedRow::new()
            .spacing(spacing::MD)
            .align_y(Alignment::Center)
            .extend(self.leading)
            .push(container(self.body).width(Fill))
            .extend(trailing);

        container(row)
            .width(Fill)
            .padding(self.padding)
            .align_y(Alignment::Center)
            .clip(true)
            .into()
    }
}

impl<'a, Message: 'a> From<ListRow<'a, Message>> for Element<'a, Message> {
    fn from(row: ListRow<'a, Message>) -> Self {
        let corners = row.corners;
        container(row.into_content())
            .width(Fill)
            .style(move |theme| control_style(theme, corners))
            .into()
    }
}

pub(crate) fn action<'a, Message: Clone + 'a>(
    row: ListRow<'a, Message>,
    on_press: Option<Message>,
    selected: bool,
) -> Element<'a, Message> {
    let corners = row.corners;
    FocusableAction::new(row.into_content())
        .width(Fill)
        .selected(selected)
        .on_press_maybe(on_press)
        .style(move |theme, state| button::Style {
            border: Border::default().rounded(corners),
            ..action_style(theme, state)
        })
        .into()
}

fn control_style(theme: &Theme, corners: Radius) -> container::Style {
    let colors = crate::theme::colors(theme);

    container::Style::default()
        .background(colors.surface)
        .border(Border::default().rounded(corners))
        .color(colors.text)
}

pub(crate) fn action_style(theme: &Theme, state: ActionState) -> button::Style {
    let colors = crate::theme::colors(theme);
    let color = if state.pressed {
        colors.selection
    } else if state.selected || state.focused {
        colors.border
    } else if state.hovered {
        colors.hover
    } else {
        colors.surface
    };

    button::Style {
        background: Some(Background::Color(color)),
        text_color: colors.text,
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}
