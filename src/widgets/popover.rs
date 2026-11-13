use iced::{
    Element, Fill, Length, Theme,
    advanced::widget::Id,
    widget::{
        Space, column, container, row, scrollable, svg, text, text::Fragment, text::IntoFragment,
        tooltip,
    },
};

use crate::icons::Icon;

use super::text::TextExt as _;
use super::{
    button::{Button, ButtonKind},
    menu::{self, overlay_item as menu_item, row_content_with},
    picker_row::PickerRow,
    popup::{AnchorMessage, Popup, PopupMessage},
    spacing,
};

const WIDTH: f32 = 240.0;

/// A trigger that opens an anchored panel: a menu of [`PopoverItem`]s, or
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
    Content(Element<'a, Message>, Vec<Id>),
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

    pub fn item(self, item: PopoverItem<'a, Message>) -> Self {
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
            Body::Content(..) => self.body = Body::Items(vec![entry]),
        }
        self
    }

    /// Shows `content` in the panel instead of a menu.
    pub fn content(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body = Body::Content(content.into(), Vec::new());
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
        let cancel_id = Id::unique();
        let confirm_id = Id::unique();
        let content = column![
            text(title).size(17).medium(),
            text(body).size(14).muted(),
            row![
                Space::new().width(Fill),
                Button::new("Cancel")
                    .kind(ButtonKind::Secondary)
                    .id(cancel_id.clone())
                    .on_press(on_cancel),
                Button::new(confirm)
                    .kind(ButtonKind::Destructive)
                    .id(confirm_id.clone())
                    .on_press(on_confirm),
            ]
            .spacing(spacing::XS),
        ]
        .spacing(spacing::SM);

        Self {
            trigger: trigger.on_press(()).into(),
            body: Body::Content(content.into(), vec![cancel_id, confirm_id]),
            width: 320.0,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let mut focus_targets = Vec::new();
        let body: Element<'a, Message> = match popover.body {
            Body::Items(items) => {
                let rows = items.into_iter().map(|entry| match entry {
                    Entry::Item(item) => item_row(item, &mut focus_targets),
                    Entry::Separator => menu::separator(),
                });
                scrollable(container(column(rows).width(Fill)).width(Fill)).into()
            }
            Body::Content(content, targets) => {
                focus_targets = targets;
                content
            }
        };
        let content = container(body)
            .width(Length::Fixed(popover.width))
            .padding(spacing::MD);

        let panel: Element<'a, Message> = container(content).style(crate::theme::overlay).into();

        Popup::new(
            popover.trigger.map(|()| AnchorMessage::Toggle),
            panel.map(PopupMessage::Close),
        )
        .focus_targets(focus_targets)
        .viewport_inset(spacing::SM)
        .into()
    }
}

enum Entry<'a, Message> {
    Item(PopoverItem<'a, Message>),
    Separator,
}

/// A row in a [`Popover`], optionally containing a trailing action.
pub struct PopoverItem<'a, Message> {
    title: Fragment<'a>,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
    leading: Option<Element<'a, Message>>,
    action: Option<ItemAction<'a, Message>>,
    tooltip: Option<Element<'a, Message>>,
    selected: bool,
}

enum ItemAction<'a, Message> {
    Row(Message),
    Trailing(&'a str, Message),
    Disabled(&'a str),
}

impl<'a, Message> PopoverItem<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            subtitle: None,
            icon: None,
            leading: None,
            action: None,
            tooltip: None,
            selected: false,
        }
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Leads the row with `leading` instead of an icon, e.g. an [`Avatar`].
    ///
    /// [`Avatar`]: super::avatar::Avatar
    pub fn leading(mut self, leading: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    pub fn on_select(mut self, message: Message) -> Self {
        self.action = Some(ItemAction::Row(message));
        self
    }

    pub fn action(mut self, label: &'a str, message: Message) -> Self {
        self.action = Some(ItemAction::Trailing(label, message));
        self
    }

    pub fn disabled_action(mut self, label: &'a str) -> Self {
        self.action = Some(ItemAction::Disabled(label));
        self
    }

    /// Wraps the row in a tooltip shown on hover — e.g. explaining a
    /// [`disabled_action`](Self::disabled_action).
    pub fn tooltip(mut self, tooltip: impl Into<Element<'a, Message>>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Marks this row as the currently active choice — shows a trailing
    /// checkmark instead of relying on the caller to encode that in the
    /// title/subtitle text.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

fn item_row<'a, Message: Clone + 'a>(
    item: PopoverItem<'a, Message>,
    focus_targets: &mut Vec<Id>,
) -> Element<'a, Message> {
    let primary = matches!(item.action.as_ref(), Some(ItemAction::Row(_))).then(Id::unique);
    focus_targets.extend(primary.iter().cloned());
    let leading = item.leading.or_else(|| {
        item.icon
            .map(|icon| icon.view().width(20).height(20).into())
    });
    let mut content = row_content_with(item.title, item.subtitle, leading);

    if item.selected {
        content = content.push(Icon::Checkmark.view().width(16).height(16).style(
            |theme: &Theme, _| svg::Style {
                color: Some(crate::theme::colors(theme).accent),
            },
        ));
    }

    let on_select = match item.action {
        Some(ItemAction::Row(message)) => Some(message),
        Some(ItemAction::Trailing(label, message)) => {
            let id = Id::unique();
            focus_targets.push(id.clone());
            content = content.push(
                Button::new(label)
                    .kind(ButtonKind::Surface)
                    .id(id)
                    .on_press(message),
            );
            None
        }
        Some(ItemAction::Disabled(label)) => {
            content = content.push(Button::new(label).kind(ButtonKind::Surface));
            None
        }
        None => None,
    };

    let mut row = menu_item(content, on_select, item.selected);
    if let Some(id) = primary {
        row = row.id(id);
    }
    let row: Element<'a, Message> = row.into();

    match item.tooltip {
        Some(tip) => tooltip(row, tip, tooltip::Position::Top)
            .style(tooltip_style)
            .padding(spacing::SM)
            .into(),
        None => row,
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    crate::theme::surface(crate::theme::deep_surface(theme))
}
