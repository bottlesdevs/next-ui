use iced::{ContentFit, Element, widget::svg};

use crate::icons::Icon;

use super::list_row::{ListRow, labels};

pub struct InfoRow<'a> {
    title: &'a str,
    description: &'a str,
    icon: Option<svg::Handle>,
}

impl<'a> InfoRow<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            description: "",
            icon: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon.handle());
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
        let row = ListRow::new(labels(info.title, info.description)).spacing(24.0);

        match info.icon {
            Some(icon) => row.leading(
                svg(icon)
                    .width(24)
                    .height(24)
                    .content_fit(ContentFit::Contain),
            ),
            None => row,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::icons::Icon;

    use super::InfoRow;

    #[test]
    fn icon_is_optional() {
        assert!(InfoRow::new("Title").icon.is_none());
        assert!(InfoRow::new("Title").icon(Icon::Timer).icon.is_some());
    }
}
