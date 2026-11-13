use iced::{
    Element, Fill, Length,
    widget::{Space, column, container, row, scrollable, text, text::IntoFragment},
};

use super::text::TextExt as _;
use super::{
    button::{Button, ButtonKind},
    list_row::{ListRow, Menu},
    picker_row::PickerRow,
    popup::{self, AnchorMessage, Popup, PopupMessage},
    spacing, text_size,
};
use crate::theme::{self, Surface};

const WIDTH: f32 = 240.0;

/// A trigger that opens an anchored panel: a menu of [`ListRow`]s, or
/// arbitrary content such as a confirmation.
///
/// Any message published from the panel closes it.
pub struct Popover<'a, Message> {
    trigger: Element<'a, ()>,
    body: Body<'a, Message>,
    width: f32,
}

enum Body<'a, Message> {
    Items(Vec<Entry<'a, Message>>),
    Content(Element<'a, Message>),
}

enum Entry<'a, Message> {
    Item(ListRow<'a, Message>),
    Separator,
}

impl<'a, Message: Clone + 'a> Popover<'a, Message> {
    pub fn new(trigger: Button<'a, ()>) -> Self {
        Self {
            trigger: trigger.on_press(()).into(),
            body: Body::Items(Vec::new()),
            width: WIDTH,
        }
    }

    pub fn row(trigger: PickerRow<'a, ()>) -> Self {
        Self {
            trigger: trigger.on_press(()).into(),
            body: Body::Items(Vec::new()),
            width: WIDTH,
        }
    }

    /// Adds a menu item: a row that's pressed as a whole or holds a trailing
    /// control, shown compact and checked if selected.
    pub fn item(self, item: ListRow<'a, Message>) -> Self {
        self.entry(Entry::Item(item))
    }

    /// Adds a line between the items before and after it, e.g. to set a
    /// destructive item apart.
    pub fn separator(self) -> Self {
        self.entry(Entry::Separator)
    }

    fn entry(mut self, entry: Entry<'a, Message>) -> Self {
        match &mut self.body {
            Body::Items(entries) => entries.push(entry),
            Body::Content(_) => self.body = Body::Items(vec![entry]),
        }
        self
    }

    /// Shows `content` in the panel instead of a menu.
    pub fn content(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body = Body::Content(content.into());
        self
    }

    /// Sets the panel's width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(1.0);
        self
    }

    /// Asks before a destructive action started from `trigger`: a title, one
    /// line explaining the consequence, Cancel and a destructive confirm.
    pub fn confirm(
        trigger: Button<'a, ()>,
        title: impl IntoFragment<'a>,
        body: impl IntoFragment<'a>,
        confirm: &'a str,
        on_confirm: Message,
        on_cancel: Message,
    ) -> Self {
        let content = column![
            text(title).size(text_size::HEADING).medium(),
            text(body).size(text_size::SMALL).muted(),
            row![
                Space::new().width(Fill),
                Button::new("Cancel")
                    .kind(ButtonKind::Secondary)
                    .on_press(on_cancel),
                Button::new(confirm)
                    .kind(ButtonKind::Destructive)
                    .on_press(on_confirm),
            ]
            .spacing(spacing::XS),
        ]
        .spacing(spacing::SM);

        Self {
            trigger: trigger.on_press(()).into(),
            body: Body::Content(content.into()),
            width: 320.0,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let body: Element<'a, Message> = match popover.body {
            Body::Items(items) => {
                let rows = items.into_iter().map(|entry| match entry {
                    Entry::Item(item) => item.menu(Menu::Floating).into(),
                    Entry::Separator => popup::separator(),
                });
                scrollable(container(column(rows).width(Fill)).width(Fill)).into()
            }
            Body::Content(content) => content,
        };
        let content = container(body)
            .width(Length::Fixed(popover.width))
            .padding(spacing::MD);

        let panel: Element<'a, Message> = container(content)
            .style(theme::surface(Surface::Overlay))
            .into();

        Popup::new(
            popover.trigger.map(|()| AnchorMessage::Toggle),
            panel.map(PopupMessage::Close),
        )
        .viewport_inset(spacing::SM)
        .into()
    }
}
