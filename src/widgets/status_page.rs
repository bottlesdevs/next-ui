use iced::{
    Center, Element, Fill,
    widget::{Column, container, text, text::IntoFragment},
};

use crate::icons::Icon;

use super::{spacing, text::TextExt as _, text_size};

const ICON_TILE: f32 = 88.0;

/// A centred page that explains an empty or unavailable view, like
/// `AdwStatusPage`: an optional icon, a title, a description and one child.
pub struct StatusPage<'a, Message> {
    icon: Option<Icon>,
    title: text::Fragment<'a>,
    description: Option<text::Fragment<'a>>,
    child: Option<Element<'a, Message>>,
}

impl<'a, Message: 'a> StatusPage<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            icon: None,
            title: title.into_fragment(),
            description: None,
            child: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Places `child` under the text, e.g. the page's actions.
    pub fn child(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.child = Some(child.into());
        self
    }
}

impl<'a, Message: 'a> From<StatusPage<'a, Message>> for Element<'a, Message> {
    fn from(page: StatusPage<'a, Message>) -> Self {
        let mut content = Column::new().spacing(spacing::SM).align_x(Center);

        if let Some(icon) = page.icon {
            content = content.push(
                container(icon.view().width(30).height(30))
                    .center(ICON_TILE)
                    .style(|theme| {
                        let colors = crate::theme::colors(theme);
                        container::Style::default()
                            .background(colors.card)
                            .border(iced::Border::default().rounded(22))
                    }),
            );
        }

        content = content.push(
            text(page.title)
                .size(text_size::PAGE_TITLE)
                .medium()
                .center(),
        );

        if let Some(description) = page.description {
            content = content.push(
                container(text(description).size(text_size::HEADING).muted().center())
                    .max_width(460),
            );
        }

        if let Some(child) = page.child {
            content = content.push(container(child).padding([spacing::XS, 0.0]));
        }

        container(content).center(Fill).into()
    }
}
