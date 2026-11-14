use iced::{
    Border, Event, Fill, Length, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{Column, button, column, container, rule, scrollable, text},
};

use crate::{Element, Theme, icons::Icon};

use super::{
    disclosure::{self, Disclosure},
    focusable_action::{FocusableAction, Interaction},
    list_row::{ListRow, Menu},
    popup::{self, AnchorMessage, Popup, PopupMessage},
    radius,
    sensitive::sensitive,
    spacing,
    switcher::Switcher,
    text::TextExt as _,
    text_size,
};

const PANEL_PADDING: [f32; 2] = [spacing::SM, spacing::SM];

/// One entry in a [`SelectorRow`]'s options: a [`choice`](Self::choice) or
/// an [`action`](Self::action), what it says, what sits in it, and what
/// pressing it does. The selector knows nothing else about it, so what an
/// entry means, e.g. a release still to download, is up to whoever builds it.
pub struct SelectorItem<'a, Message> {
    kind: Kind,
    label: String,
    detail: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    on_press: Option<Message>,
    selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Choice,
    Action,
}

impl<'a, Message> SelectorItem<'a, Message> {
    /// A value to pick: pressing it closes the options, and it can be the
    /// [`selected`](Self::selected) one.
    pub fn choice(label: impl Into<String>) -> Self {
        Self::new(Kind::Choice, label)
    }

    /// Something to do that picks no value, e.g. starting a download: the
    /// options stay open, so whatever it starts stays in view.
    pub fn action(label: impl Into<String>) -> Self {
        Self::new(Kind::Action, label)
    }

    fn new(kind: Kind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            detail: None,
            trailing: None,
            on_press: None,
            selected: false,
        }
    }

    /// Adds a widget under the label, e.g. a [`ProgressBar`].
    ///
    /// [`ProgressBar`]: super::progress_bar::ProgressBar
    pub fn detail(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        self.detail = Some(widget.into());
        self
    }

    /// Adds a widget at the item's end, e.g. an icon, or a button showing
    /// what pressing the item does. The item is one entry, so what sits in it
    /// takes no presses or focus of its own.
    pub fn trailing(mut self, widget: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(widget.into());
        self
    }

    /// Without one, the item is shown unavailable.
    pub fn on_press(self, message: Message) -> Self {
        self.on_press_maybe(Some(message))
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Marks the current choice, whose label the selector's row shows. Only
    /// a choice can be selected; an action ignores it.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected && self.kind == Kind::Choice;
        self
    }
}

/// An in-flow row that selects one value from a connected panel of items,
/// grouped under optional titles.
pub struct SelectorRow<'a, Message> {
    title: &'a str,
    groups: Vec<(Option<&'a str>, Vec<SelectorItem<'a, Message>>)>,
    placeholder: &'a str,
    icon: Option<Icon>,
    popup: bool,
    switch: Option<Switcher<'a, Message>>,
}

impl<'a, Message> SelectorRow<'a, Message> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            groups: Vec::new(),
            placeholder: "",
            icon: None,
            popup: false,
            switch: None,
        }
    }

    /// Adds `items`, under `title` if one is given; a separator sets each
    /// group after the first apart. A group without items isn't shown.
    pub fn group(
        mut self,
        title: impl Into<Option<&'a str>>,
        items: impl IntoIterator<Item = SelectorItem<'a, Message>>,
    ) -> Self {
        let items = items.into_iter().collect::<Vec<_>>();
        if !items.is_empty() {
            self.groups.push((title.into(), items));
        }
        self
    }

    /// Opens the options over the content below instead of pushing it down,
    /// e.g. in a two-column settings grid.
    pub fn popup(mut self) -> Self {
        self.popup = true;
        self
    }

    /// Adds a switch before the caret that turns the setting on or off, e.g.
    /// a component whose release the options pick.
    pub fn switch(mut self, value: bool, on_toggle: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.switch = Some(Switcher::new(value).on_toggle_maybe(on_toggle));
        self
    }

    /// Shown in the row while no item is selected.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SelectorRow<'a, Message>> for Element<'a, Message> {
    fn from(selector: SelectorRow<'a, Message>) -> Self {
        let items = || selector.groups.iter().flat_map(|(_, items)| items);
        let value = items().find(|item| item.selected).map_or_else(
            || selector.placeholder.to_owned(),
            |item| item.label.clone(),
        );
        // Worth opening only if some item can be pressed.
        let enabled = items().any(|item| item.on_press.is_some());
        let header = header(
            selector.title,
            value,
            selector.icon,
            selector.switch.map(Element::from),
        );
        let mut list = Column::new().width(Fill);
        for (index, (title, items)) in selector.groups.into_iter().enumerate() {
            if index > 0 {
                list = list.push(popup::separator());
            }
            if let Some(title) = title {
                list = list.push(section_heading(title));
            }
            for item in items {
                let kind = item.kind;
                let label = text(item.label).size(text_size::DENSE);
                let body: Element<'a, PopupMessage<Message>> = match item.detail {
                    Some(detail) => column![label, detail.map(PopupMessage::Emit)]
                        .spacing(spacing::XS)
                        .into(),
                    None => label.into(),
                };
                let mut row = ListRow::new(body)
                    .menu(Menu::Attached)
                    .padding([spacing::SM, spacing::MD])
                    .selected(item.selected)
                    .on_press_maybe(item.on_press.map(|message| match kind {
                        Kind::Choice => PopupMessage::Close(message),
                        Kind::Action => PopupMessage::Emit(message),
                    }));
                if let Some(trailing) = item.trailing {
                    row = row.trailing(trailing.map(PopupMessage::Emit));
                }
                list = list.push(row);
            }
        }
        let popup = selector.popup;
        let list = scrollable(container(list).width(Fill).padding(PANEL_PADDING)).width(Fill);
        // The options continue the row: a hairline marks where the row ends.
        let list = column![rule::horizontal(1), list,];
        let panel = container(list).width(Fill).style(move |theme| {
            container::Style::default()
                .background(theme.colors().raised)
                .border(Border::default().rounded(iced::border::bottom(if popup {
                    radius::BOX
                } else {
                    radius::ROW
                })))
        });

        // Disabled as a whole, header switch included.
        if !popup {
            return sensitive(enabled, Disclosure::new(header, panel).dismissible());
        }

        let anchor = FocusableAction::new(header.map(AnchorMessage::Emit))
            .width(Fill)
            .on_press_maybe(Some(AnchorMessage::Toggle))
            .without_frame()
            .style(|theme, _| button::Style {
                text_color: theme.colors().text,
                ..button::Style::default()
            });

        sensitive(
            enabled,
            Element::new(PopupSelector {
                popup: Popup::new(anchor, panel)
                    .match_anchor_width()
                    .attached()
                    .into(),
            }),
        )
    }
}

fn section_heading<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(text_size::FOOTNOTE).muted())
        .padding([spacing::XS, spacing::MD])
        .into()
}

/// The row of either mode: the title, the value under it, an optional
/// switch, and room for the caret.
fn header<'a, Message: 'a>(
    title: &'a str,
    value: String,
    icon: Option<Icon>,
    switch: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut row = ListRow::labeled(title).subtitle(value);
    if let Some(icon) = icon {
        row = row.subtitle_icon(icon);
    }
    if let Some(switch) = switch {
        row = row.trailing(switch);
    }
    row.into_disclosure_content()
}

/// A selector whose options open over the content below: the row and its
/// caret around a [`Popup`] the row toggles.
struct PopupSelector<'a, Message> {
    popup: Element<'a, Message>,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for PopupSelector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.popup.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.popup.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.popup.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.popup.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.popup.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.popup.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.popup
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.popup.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.popup
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let open = popup::is_open(tree);
        let bounds = layout.bounds();
        let interaction = popup::anchor(tree).state.downcast_ref::<Interaction>();
        disclosure::draw_header(
            renderer,
            theme,
            bounds,
            interaction.state(true, open, bounds, cursor),
            open,
        );
        self.popup
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
        disclosure::draw_caret(renderer, theme, bounds, open);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        self.popup
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}
