use iced::{
    Alignment, Element,
    widget::{
        Space, column, row, text,
        text::{Fragment, IntoFragment},
    },
};

use crate::icons::Icon;

use super::{
    list_row::{self, ListRow},
    progress_ring::ProgressRing,
    spacing,
    text::TextExt as _,
};

enum State<Message> {
    Ready(Message),
    Disabled,
    Progress(f32),
}

/// A row that activates as a whole, like `AdwActionRow`: a title and
/// description between any number of leading and trailing widgets.
///
/// Presses on a leading or trailing control go to that control, not the row.
pub struct ActionRow<'a, Message> {
    title: Fragment<'a>,
    description: Fragment<'a>,
    icon: Option<Icon>,
    title_suffix: Vec<Element<'a, Message>>,
    row: ListRow<'a, Message>,
    state: State<Message>,
    selected: bool,
}

impl<'a, Message: 'a> ActionRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, action: impl Into<Option<Message>>) -> Self {
        Self {
            title: title.into_fragment(),
            description: "".into_fragment(),
            icon: None,
            title_suffix: Vec::new(),
            row: ListRow::new(Space::new()),
            state: action.into().map_or(State::Disabled, State::Ready),
            selected: false,
        }
    }

    /// A row showing `progress` in a ring after its trailing widgets.
    pub fn progress(title: impl IntoFragment<'a>, progress: f32) -> Self {
        Self {
            state: State::Progress(progress),
            ..Self::new(title, None)
        }
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = description.into_fragment();
        self
    }

    /// Adds a widget on the title line after the title, e.g. a version or a
    /// [`Tag`].
    ///
    /// [`Tag`]: super::tag::Tag
    pub fn title_suffix(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        self.title_suffix.push(widget.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds a widget before the title, e.g. an [`Avatar`].
    ///
    /// [`Avatar`]: super::avatar::Avatar
    pub fn leading(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.row = self.row.leading(control);
        self
    }

    /// Adds a widget after the title, e.g. a button or a chevron for a row
    /// that navigates.
    pub fn trailing(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.row = self.row.trailing(control);
        self
    }

    pub(crate) fn grouped(mut self, corners: iced::border::Radius) -> Self {
        self.row = self.row.grouped(corners);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        let mut description = row![].spacing(spacing::SM).align_y(Alignment::Center);

        if let Some(icon) = action.icon {
            description = description.push(
                icon.view()
                    .width(list_row::BODY_SIZE)
                    .height(list_row::BODY_SIZE),
            );
        }

        let has_description = action.icon.is_some() || !action.description.is_empty();
        description = description.push(text(action.description).size(list_row::BODY_SIZE).muted());

        let title = row![text(action.title).label().medium()]
            .extend(action.title_suffix)
            .spacing(spacing::SM)
            .align_y(Alignment::Center);
        let mut labels = column![title].spacing(spacing::XS);
        if has_description {
            labels = labels.push(description);
        }
        let mut row = action.row.body(labels);
        let on_press = match action.state {
            State::Ready(message) => Some(message),
            State::Disabled => None,
            State::Progress(progress) => {
                row = row.trailing(ProgressRing::new(progress));
                None
            }
        };

        list_row::action(row, on_press, action.selected)
    }
}
