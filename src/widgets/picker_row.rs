use iced::{Element, widget::text::IntoFragment};

use crate::icons::Icon;

use super::list_row::ListRow;

/// A pressable row that picks a file, shown by its folder icon.
pub struct PickerRow<'a, Message>(ListRow<'a, Message>);

impl<'a, Message: 'a> PickerRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self(
            ListRow::labeled(title)
                .subtitle("")
                .trailing(Icon::Folder.view().width(25).height(20))
                .on_press_maybe(None),
        )
    }

    pub fn description(self, description: impl IntoFragment<'a>) -> Self {
        Self(self.0.subtitle(description))
    }

    pub fn on_press(self, on_press: Message) -> Self {
        Self(self.0.on_press(on_press))
    }

    pub fn on_press_maybe(self, on_press: Option<Message>) -> Self {
        Self(self.0.on_press_maybe(on_press))
    }
}

impl<'a, Message: Clone + 'a> From<PickerRow<'a, Message>> for Element<'a, Message> {
    fn from(picker: PickerRow<'a, Message>) -> Self {
        picker.0.into()
    }
}

impl<'a, Message> From<PickerRow<'a, Message>> for ListRow<'a, Message> {
    fn from(picker: PickerRow<'a, Message>) -> Self {
        picker.0
    }
}
