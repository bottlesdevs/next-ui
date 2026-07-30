use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, row, text},
};

use super::{row_surface::RowSurface, style, switcher::Switcher};

pub struct SwitcherRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    value: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message> SwitcherRow<'a, Message> {
    pub fn new(value: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        Self {
            title: "",
            description: "",
            value,
            on_toggle: Box::new(on_toggle),
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }
}

impl<'a, Message: Clone + 'a> From<SwitcherRow<'a, Message>> for Element<'a, Message> {
    fn from(switcher: SwitcherRow<'a, Message>) -> Self {
        let labels = column![
            text(switcher.title).size(18).style(text::base),
            text(switcher.description).size(16).style(style::muted_text),
        ]
        .spacing(4);

        RowSurface::new(
            container(
                row![
                    labels,
                    Space::new().width(Fill),
                    Switcher::new(switcher.value, switcher.on_toggle),
                ]
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([18, 24]),
        )
        .into()
    }
}
