use iced::{
    Alignment, Background, Border, Element, Fill, Padding, Theme,
    widget::{Row as IcedRow, Space, column, container, text, text::IntoFragment},
};

use super::{
    control::{Control, State, Style},
    spacing,
    text::TextExt as _,
};

pub(crate) const BODY_SIZE: f32 = 16.0;
pub(crate) const STANDARD_PADDING: Padding = Padding {
    top: spacing::MD,
    right: spacing::MD,
    bottom: spacing::MD,
    left: spacing::LG,
};

/// A semantic row recipe. Its visual control is built only when the row is
/// placed standalone or lowered by a [`super::row_group::RowGroup`].
pub struct ListRow<'a, Message> {
    body: Element<'a, Message>,
    leading: Vec<Element<'a, Message>>,
    trailing: Vec<Element<'a, Message>>,
    enabled: bool,
    selected: bool,
    on_press: Option<Message>,
    focus_first: bool,
    padding: Padding,
}

pub(crate) struct Content<'a, Message> {
    pub(crate) element: Element<'a, Message>,
    pub(crate) enabled: bool,
    pub(crate) selected: bool,
    pub(crate) on_press: Option<Message>,
    pub(crate) focus_first: bool,
    /// Index of the disclosure slot in the inner iced row, when present.
    pub(crate) disclosure_index: Option<usize>,
}

pub(crate) fn labels<'a, Message: 'a>(
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
            enabled: true,
            selected: false,
            on_press: None,
            focus_first: false,
            padding: STANDARD_PADDING,
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

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub(crate) fn focus_first(mut self) -> Self {
        self.focus_first = true;
        self
    }

    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub(crate) fn into_content(self) -> Content<'a, Message> {
        self.lower(false)
    }

    pub(crate) fn into_disclosure_content(self) -> Content<'a, Message> {
        self.lower(true)
    }

    pub(crate) fn into_control(self, parent_enabled: bool) -> Control<'a, Message> {
        let Content {
            element,
            enabled,
            selected,
            on_press,
            focus_first,
            ..
        } = self.into_content();
        let mut control = Control::new(element)
            .width(Fill)
            .sensitive(parent_enabled && enabled)
            .selected(selected)
            .on_press_maybe(on_press)
            .style(style);

        if focus_first {
            control = control.focus_first_descendant();
        }

        control
    }

    fn lower(self, disclosure: bool) -> Content<'a, Message> {
        let disclosure_index = disclosure.then_some(self.leading.len() + 1);
        let mut trailing = self.trailing;

        if disclosure {
            trailing.insert(0, Space::new().width(20).height(20).into());
        }

        let row = IcedRow::new()
            .spacing(spacing::MD)
            .align_y(Alignment::Center)
            .extend(self.leading)
            .push(container(self.body).width(Fill))
            .extend(trailing);

        Content {
            element: container(row)
                .width(Fill)
                .padding(self.padding)
                .align_y(Alignment::Center)
                .clip(true)
                .into(),
            enabled: self.enabled,
            selected: self.selected,
            on_press: self.on_press,
            focus_first: self.focus_first,
            disclosure_index,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ListRow<'a, Message>> for Element<'a, Message> {
    fn from(row: ListRow<'a, Message>) -> Self {
        row.into_control(true).into()
    }
}

pub(crate) fn style(theme: &Theme, state: State) -> Style {
    let palette = theme.extended_palette();
    let color = if state.pressed {
        palette.background.stronger.color
    } else if state.selected || state.expanded || state.focus_within {
        palette.background.neutral.color
    } else if state.hovered {
        palette.background.strong.color
    } else {
        palette.background.weak.color
    };

    Style {
        background: Some(Background::Color(color)),
        text_color: theme.palette().text,
        border: Border::default().rounded(6),
        foreground: (!state.sensitive).then_some(Background::Color(crate::theme::SCRIM)),
        ..Style::default()
    }
}
