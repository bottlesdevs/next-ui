use iced::{
    Element,
    widget::text::{Fragment, IntoFragment},
};

use crate::icons::Icon;

use super::list_row::ListRow;

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
        let row = ListRow::labeled(info.title).subtitle(info.description);
        match info.icon {
            Some(icon) => row.subtitle_icon(icon),
            None => row,
        }
    }
}
