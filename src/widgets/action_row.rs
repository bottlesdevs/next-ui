use iced::{
    Alignment, Element,
    widget::{column, row, text},
};

use crate::icons::Icon;

use super::{
    list_row::{self, ListRow},
    progress_ring::ProgressRing,
    spacing,
    text::TextExt as _,
};

#[derive(Debug, Clone)]
pub enum State<Message> {
    Ready(Message),
    Disabled,
    /// In-flight state (e.g. downloading a component, linking an account), `0.0..=1.0`.
    Progress(f32),
}

pub struct ActionRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    icon: Option<Icon>,
    state: State<Message>,
    selected: bool,
}

impl<'a, Message> ActionRow<'a, Message> {
    pub fn new(title: &'a str, state: State<Message>) -> Self {
        Self {
            title,
            description: "",
            icon: None,
            state,
            selected: false,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
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

        description = description.push(text(action.description).size(list_row::BODY_SIZE).muted());

        let labels = column![text(action.title).label().medium(), description].spacing(spacing::XS);
        let trailing: Element<'a, Message> = match &action.state {
            State::Progress(progress) => ProgressRing::new(*progress).into(),
            State::Ready(_) | State::Disabled => {
                Icon::Arrow.view().rotation(std::f32::consts::PI).into()
            }
        };
        let on_press = match action.state {
            State::Ready(message) => Some(message),
            State::Disabled | State::Progress(_) => None,
        };

        list_row::action(
            ListRow::new(labels).trailing(trailing),
            on_press,
            action.selected,
        )
    }
}
