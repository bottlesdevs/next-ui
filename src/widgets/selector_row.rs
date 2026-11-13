use iced::{
    Border, Element, Event, Fill, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    widget::{Column, button, column, container, scrollable, text},
};

use crate::icons::Icon;

use super::{
    disclosure::{self, Disclosure},
    focusable_action::{FocusableAction, Interaction},
    list_row::{ListRow, Menu},
    popup::{self, AnchorMessage, Popup, PopupMessage},
    radius, spacing,
    switcher::Switcher,
    text::TextExt as _,
    text_size,
};

const PANEL_PADDING: [f32; 2] = [spacing::SM, spacing::SM];

/// An in-flow row that selects one value from a connected option panel.
pub struct SelectorRow<'a, T, Message> {
    title: &'a str,
    options: Vec<T>,
    selected: Option<T>,
    label: Box<dyn Fn(&T) -> String + 'a>,
    on_selected: Option<Box<dyn Fn(T) -> Message + 'a>>,
    placeholder: &'a str,
    icon: Option<Icon>,
    options_title: Option<&'a str>,
    sections: Vec<(&'a str, Vec<Element<'a, Message>>)>,
    popup: bool,
    switch: Option<Switcher<'a, Message>>,
}

impl<'a, T: Clone + ToString + 'a, Message> SelectorRow<'a, T, Message> {
    pub fn new(title: &'a str, options: &[T], selected: Option<T>) -> Self {
        Self {
            title,
            options: options.to_vec(),
            selected,
            label: Box::new(ToString::to_string),
            on_selected: None,
            placeholder: "",
            icon: None,
            options_title: None,
            sections: Vec::new(),
            popup: false,
            switch: None,
        }
    }

    /// Heads the options with `title` when there are any, e.g. to set them
    /// apart from the sections under them.
    pub fn options_title(mut self, title: &'a str) -> Self {
        self.options_title = Some(title);
        self
    }

    /// Lists `rows` under the options with a `title`, e.g. releases that can
    /// be downloaded. Unlike picking an option, their messages keep the
    /// options open. A section without rows isn't shown.
    pub fn section(
        mut self,
        title: &'a str,
        rows: impl IntoIterator<Item = Element<'a, Message>>,
    ) -> Self {
        let rows = rows.into_iter().collect::<Vec<_>>();
        if !rows.is_empty() {
            self.sections.push((title, rows));
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

    pub fn label(mut self, label: impl Fn(&T) -> String + 'a) -> Self {
        self.label = Box::new(label);
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_selected(mut self, on_selected: impl Fn(T) -> Message + 'a) -> Self {
        self.on_selected = Some(Box::new(on_selected));
        self
    }
}

impl<'a, T, Message> From<SelectorRow<'a, T, Message>> for Element<'a, Message>
where
    T: Clone + PartialEq + ToString + 'a,
    Message: Clone + 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let selected = selector
            .selected
            .as_ref()
            .filter(|selected| selector.options.contains(selected));
        let label = &selector.label;
        let value = selected.map_or_else(|| selector.placeholder.to_owned(), label);
        // Sections, e.g. releases to download, are worth opening for even
        // with no option to pick.
        let enabled = selector.on_selected.is_some()
            && !(selector.options.is_empty() && selector.sections.is_empty());
        let header = header(
            selector.title,
            value,
            selector.icon,
            selector.switch.map(Element::from),
        );
        let rows = selector.options.iter().map(|option| {
            let message = selector
                .on_selected
                .as_ref()
                .map(|on_selected| PopupMessage::Close(on_selected(option.clone())));

            ListRow::new(text(label(option)).size(text_size::DENSE))
                .menu(Menu::Attached)
                .padding([spacing::SM, spacing::MD])
                .selected(selected == Some(option))
                .on_press_maybe(message)
                .into()
        });
        let mut list = Column::new().width(Fill);
        if let Some(title) = selector
            .options_title
            .filter(|_| !selector.options.is_empty())
        {
            list = list.push(section_heading(title));
        }
        list = list.extend(rows);
        let mut above = !selector.options.is_empty();
        for (title, rows) in selector.sections {
            if above {
                list = list.push(popup::separator());
            }
            above = true;
            list = list
                .push(section_heading(title))
                .extend(rows.into_iter().map(|row| {
                    container(row.map(PopupMessage::Emit))
                        .padding([spacing::XS, spacing::MD])
                        .into()
                }));
        }
        let popup = selector.popup;
        let list = scrollable(container(list).width(Fill).padding(PANEL_PADDING))
            .width(Fill)
            .style(crate::theme::scrollbar);
        // The options continue the row: a hairline marks where the row ends.
        let list = column![
            container(iced::widget::Space::new().height(1))
                .width(Fill)
                .style(|theme| {
                    container::Style::default().background(crate::theme::colors(theme).selected)
                }),
            list,
        ];
        let panel = container(list).width(Fill).style(move |theme| {
            container::Style::default()
                .background(crate::theme::colors(theme).raised)
                .border(Border::default().rounded(iced::border::bottom(if popup {
                    radius::BOX
                } else {
                    radius::ROW
                })))
        });

        if !popup {
            return Disclosure::new(header, panel)
                .enabled(enabled)
                .dismissible()
                .into();
        }

        let anchor = FocusableAction::new(header.map(AnchorMessage::Emit))
            .width(Fill)
            .on_press_maybe(enabled.then_some(AnchorMessage::Toggle))
            .style(|theme, _| button::Style {
                text_color: crate::theme::colors(theme).text,
                ..button::Style::default()
            });

        Element::new(PopupSelector {
            popup: Popup::new(anchor, panel)
                .enabled(enabled)
                .match_anchor_width()
                .attached()
                .into(),
            enabled,
        })
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
    enabled: bool,
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
            interaction.state(self.enabled, open, bounds, cursor),
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
