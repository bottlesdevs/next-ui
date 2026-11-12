use iced::{
    Alignment, Element,
    widget::{
        column, row, text,
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

pub struct ActionRow<'a, Message> {
    title: Fragment<'a>,
    description: Fragment<'a>,
    icon: Option<Icon>,
    leading: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    state: State<Message>,
    selected: bool,
}

impl<'a, Message> ActionRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, action: impl Into<Option<Message>>) -> Self {
        Self {
            title: title.into_fragment(),
            description: "".into_fragment(),
            icon: None,
            leading: None,
            trailing: None,
            state: action.into().map_or(State::Disabled, State::Ready),
            selected: false,
        }
    }

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

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Leads the row with `leading`, e.g. an [`Avatar`].
    ///
    /// [`Avatar`]: super::avatar::Avatar
    pub fn leading(mut self, leading: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// Ends the row with `trailing` instead of the arrow, e.g. a delete
    /// button.
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        action.lower()
    }
}

impl<'a, Message: Clone + 'a> ActionRow<'a, Message> {
    fn lower(self) -> Element<'a, Message> {
        let mut description = row![].spacing(spacing::SM).align_y(Alignment::Center);

        if let Some(icon) = self.icon {
            description = description.push(
                icon.view()
                    .width(list_row::BODY_SIZE)
                    .height(list_row::BODY_SIZE),
            );
        }

        description = description.push(text(self.description).size(list_row::BODY_SIZE).muted());

        let labels = column![text(self.title).label().medium(), description].spacing(spacing::XS);
        let (arrow, action): (Element<'a, Message>, _) = match self.state {
            State::Ready(message) => (
                Icon::Arrow.view().rotation(std::f32::consts::PI).into(),
                Some(message),
            ),
            State::Disabled => (
                Icon::Arrow.view().rotation(std::f32::consts::PI).into(),
                None,
            ),
            State::Progress(progress) => (ProgressRing::new(progress).into(), None),
        };
        let mut row = ListRow::new(labels).trailing(self.trailing.unwrap_or(arrow));
        if let Some(leading) = self.leading {
            row = row.leading(leading);
        }
        list_row::action(row, action, self.selected)
    }
}
