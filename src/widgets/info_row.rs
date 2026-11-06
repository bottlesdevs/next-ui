use iced::{
    Center, Element,
    widget::{
        column, row, text,
        text::{Fragment, IntoFragment},
    },
};

use crate::icons::Icon;

use super::{
    list_row::{self, ListRow},
    spacing,
    text::TextExt as _,
};

pub struct InfoRow<'a> {
    title: Fragment<'a>,
    description: Fragment<'a>,
    icon: Option<Icon>,
}

impl<'a> InfoRow<'a> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            description: "".into_fragment(),
            icon: None,
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
}

impl<'a, Message: Clone + 'a> From<InfoRow<'a>> for Element<'a, Message> {
    fn from(info: InfoRow<'a>) -> Self {
        ListRow::from(info).into()
    }
}

impl<'a, Message: 'a> From<InfoRow<'a>> for ListRow<'a, Message> {
    fn from(info: InfoRow<'a>) -> Self {
        let mut description = row![].spacing(spacing::SM).align_y(Center);
        if let Some(icon) = info.icon {
            description = description.push(
                icon.view()
                    .width(list_row::BODY_SIZE)
                    .height(list_row::BODY_SIZE),
            );
        }
        description = description.push(text(info.description).size(list_row::BODY_SIZE).muted());
        ListRow::new(column![text(info.title).label().medium(), description].spacing(spacing::XS))
    }
}
