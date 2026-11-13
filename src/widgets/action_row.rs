use iced::{Element, widget::text::IntoFragment};

use crate::icons::Icon;

use super::{list_row::ListRow, progress_ring::ProgressRing};

/// A row that activates as a whole, like `AdwActionRow`: a title and
/// description between any number of leading and trailing widgets.
///
/// Presses on a leading or trailing control go to that control, not the row.
pub struct ActionRow<'a, Message> {
    row: ListRow<'a, Message>,
    progress: Option<f32>,
}

impl<'a, Message: 'a> ActionRow<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, action: impl Into<Option<Message>>) -> Self {
        Self {
            row: ListRow::labeled(title).on_press_maybe(action.into()),
            progress: None,
        }
    }

    /// A row showing `progress` in a ring after its trailing widgets.
    pub fn progress(title: impl IntoFragment<'a>, progress: f32) -> Self {
        Self {
            progress: Some(progress),
            ..Self::new(title, None)
        }
    }

    /// Sets the line under the title; an empty one leaves the row a single
    /// line unless it has an icon.
    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        let description = description.into_fragment();
        if !description.is_empty() {
            self.row = self.row.subtitle(description);
        }
        self
    }

    /// Adds a widget on the title line after the title, e.g. a version or a
    /// [`Tag`].
    ///
    /// [`Tag`]: super::tag::Tag
    pub fn title_suffix(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        self.row = self.row.title_suffix(widget);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.row = self.row.subtitle_icon(icon);
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

    pub fn selected(mut self, selected: bool) -> Self {
        self.row = self.row.selected(selected);
        self
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        ListRow::from(action).into()
    }
}

impl<'a, Message: 'a> From<ActionRow<'a, Message>> for ListRow<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        match action.progress {
            Some(progress) => action.row.trailing(ProgressRing::new(progress)),
            None => action.row,
        }
    }
}
