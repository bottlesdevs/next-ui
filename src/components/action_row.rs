use iced::{
    Alignment, ContentFit, Element,
    widget::{column, row, svg, text},
};

use crate::icons::Icon;

use super::{list_row::ListRow, spacing, text::TextExt as _};

#[derive(Debug, Clone)]
pub enum State<Message> {
    Ready(Message),
    Disabled,
}

pub struct ActionRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    icon: Option<Icon>,
    state: State<Message>,
}

impl<'a, Message> ActionRow<'a, Message> {
    pub fn new(title: &'a str, state: State<Message>) -> Self {
        Self {
            title,
            description: "",
            icon: None,
            state,
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
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        ListRow::from(action).into()
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for ListRow<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        let mut description = row![].spacing(spacing::SM).align_y(Alignment::Center);

        if let Some(icon) = action.icon {
            description = description.push(
                svg(icon.handle())
                    .width(24)
                    .height(24)
                    .content_fit(ContentFit::Contain),
            );
        }

        description = description.push(text(action.description).detail().muted());

        let labels = column![text(action.title).label(), description].spacing(spacing::XS);

        match action.state {
            State::Ready(message) => ListRow::new(labels)
                .trailing(Icon::Arrow.rotated(std::f32::consts::PI))
                .on_press(message),
            State::Disabled => ListRow::new(labels)
                .trailing(Icon::Arrow.rotated(std::f32::consts::PI))
                .enabled(false),
        }
    }
}
