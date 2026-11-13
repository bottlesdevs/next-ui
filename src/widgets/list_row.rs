use iced::{
    Alignment, Background, Border, Element, Fill, Padding, Theme,
    border::Radius,
    widget::{
        Column, Row as IcedRow, Space, button, container, row, text,
        text::{Fragment, IntoFragment},
    },
};

use crate::icons::Icon;

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

/// The toolkit's one row: leading widgets, a title and subtitle (or any
/// body), trailing widgets, and optionally activation as a whole.
///
/// Presses on a leading or trailing control go to that control, not the row.
pub struct ListRow<'a, Message> {
    body: Body<'a, Message>,
    leading: Vec<Element<'a, Message>>,
    trailing: Vec<Element<'a, Message>>,
    padding: Padding,
    corners: Radius,
    pressable: bool,
    on_press: Option<Message>,
    selected: bool,
}

enum Body<'a, Message> {
    Custom(Element<'a, Message>),
    Labels(Labels<'a, Message>),
}

struct Labels<'a, Message> {
    title: Fragment<'a>,
    title_suffix: Vec<Element<'a, Message>>,
    subtitle: Option<Subtitle<'a, Message>>,
    centered: bool,
    dimmed: bool,
}

enum Subtitle<'a, Message> {
    Text {
        text: Fragment<'a>,
        icon: Option<Icon>,
    },
    Widget(Element<'a, Message>),
}

impl<'a, Message: 'a> ListRow<'a, Message> {
    /// A row whose body is `body` as given.
    pub fn new(body: impl Into<Element<'a, Message>>) -> Self {
        Self::with_body(Body::Custom(body.into()))
    }

    /// A row whose body is a title, with an optional subtitle line below it.
    pub fn labeled(title: impl IntoFragment<'a>) -> Self {
        Self::with_body(Body::Labels(Labels {
            title: title.into_fragment(),
            title_suffix: Vec::new(),
            subtitle: None,
            centered: false,
            dimmed: false,
        }))
    }

    fn with_body(body: Body<'a, Message>) -> Self {
        Self {
            body,
            leading: Vec::new(),
            trailing: Vec::new(),
            padding: STANDARD_PADDING,
            corners: Radius::new(RADIUS),
            pressable: false,
            on_press: None,
            selected: false,
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

    /// Adds a widget on the title line after the title, e.g. a [`Tag`].
    ///
    /// [`Tag`]: super::tag::Tag
    pub fn title_suffix(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            labels.title_suffix.push(widget.into());
        }
        self
    }

    /// Sets the subtitle line's text, keeping its icon.
    pub fn subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            let icon = match labels.subtitle.take() {
                Some(Subtitle::Text { icon, .. }) => icon,
                _ => None,
            };
            labels.subtitle = Some(Subtitle::Text {
                text: subtitle.into_fragment(),
                icon,
            });
        }
        self
    }

    /// Shows `icon` before the subtitle line's text.
    pub fn subtitle_icon(mut self, icon: Icon) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            let text = match labels.subtitle.take() {
                Some(Subtitle::Text { text, .. }) => text,
                _ => "".into_fragment(),
            };
            labels.subtitle = Some(Subtitle::Text {
                text,
                icon: Some(icon),
            });
        }
        self
    }

    /// Puts `widget` on the subtitle line instead of text, e.g. a text field.
    pub fn subtitle_widget(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            labels.subtitle = Some(Subtitle::Widget(widget.into()));
        }
        self
    }

    /// Makes the whole row pressable, like `AdwActionRow`.
    pub fn on_press(self, on_press: Message) -> Self {
        self.on_press_maybe(Some(on_press))
    }

    /// Makes the whole row pressable; with `None` it shows as disabled.
    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.pressable = true;
        self.on_press = on_press;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Centres the labels, for a row whose controls flank them.
    pub(crate) fn centered(mut self) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            labels.centered = true;
        }
        self
    }

    /// Mutes the title, for a row whose controls are all disabled.
    pub(crate) fn dimmed(mut self, dimmed: bool) -> Self {
        if let Body::Labels(labels) = &mut self.body {
            labels.dimmed = dimmed;
        }
        self
    }

    /// Fits the row into a boxed group: rounded as its place there gives it,
    /// and tighter, since the group's hairlines already part the rows.
    pub(crate) fn grouped(mut self, corners: Radius) -> Self {
        self.corners = corners;
        self.padding.top = spacing::SM;
        self.padding.bottom = spacing::SM;
        self
    }

    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// The row's content without its own background, for a container that
    /// draws the background and a disclosure caret in the reserved slot.
    pub(crate) fn into_disclosure_content(self) -> Element<'a, Message> {
        self.into_content(true)
    }

    fn into_content(self, disclosure: bool) -> Element<'a, Message> {
        let mut trailing = self.trailing;

        if disclosure {
            trailing.push(Space::new().width(20).height(20).into());
        }

        let body = match self.body {
            Body::Custom(body) => body,
            Body::Labels(labels) => labels.into_element(),
        };

        let row = IcedRow::new()
            .spacing(spacing::MD)
            .align_y(Alignment::Center)
            .extend(self.leading)
            .push(container(body).width(Fill))
            .extend(trailing);

        container(row)
            .width(Fill)
            .padding(self.padding)
            .align_y(Alignment::Center)
            .clip(true)
            .into()
    }
}

impl<'a, Message: 'a> Labels<'a, Message> {
    /// The one place a row's title and subtitle are laid out.
    fn into_element(self) -> Element<'a, Message> {
        let mut title = text(self.title).label().medium();
        if self.dimmed {
            title = title.muted();
        }
        let title: Element<'a, Message> = if self.title_suffix.is_empty() {
            title.into()
        } else {
            row![title]
                .extend(self.title_suffix)
                .spacing(spacing::SM)
                .align_y(Alignment::Center)
                .into()
        };

        let subtitle = self.subtitle.map(|subtitle| match subtitle {
            Subtitle::Text { text: line, icon } => {
                let line = text(line).size(BODY_SIZE).muted();
                match icon {
                    Some(icon) => row![icon.view().width(BODY_SIZE).height(BODY_SIZE), line]
                        .spacing(spacing::SM)
                        .align_y(Alignment::Center)
                        .into(),
                    None => line.into(),
                }
            }
            Subtitle::Widget(widget) => widget,
        });

        let mut labels = Column::new().push(title);
        if let Some(subtitle) = subtitle {
            labels = labels.push(subtitle);
        }
        labels
            .width(Fill)
            .align_x(if self.centered {
                Alignment::Center
            } else {
                Alignment::Start
            })
            .spacing(spacing::XS)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<ListRow<'a, Message>> for Element<'a, Message> {
    fn from(mut row: ListRow<'a, Message>) -> Self {
        let corners = row.corners;
        let (pressable, on_press, selected) = (row.pressable, row.on_press.take(), row.selected);
        let content = row.into_content(false);

        if pressable {
            FocusableAction::new(content)
                .width(Fill)
                .selected(selected)
                .on_press_maybe(on_press)
                .style(move |theme, state| button::Style {
                    border: Border::default().rounded(corners),
                    ..action_style(theme, state)
                })
                .into()
        } else {
            container(content)
                .width(Fill)
                .style(move |theme| control_style(theme, corners))
                .into()
        }
    }
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
