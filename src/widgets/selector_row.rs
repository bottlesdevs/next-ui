use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Point, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{Column, button, column, container, row, scrollable, text},
};

use crate::icons::Icon;

use super::{
    draw_caret,
    focusable_action::{ActionState, FocusableAction, Interaction},
    list_row::{self, ListRow, Menu},
    popup::{self, AnchorMessage, Popup, PopupMessage},
    spacing,
    switcher::Switcher,
    text::TextExt as _,
};

const PANEL_PADDING: [f32; 2] = [spacing::SM, spacing::SM];
/// Options read smaller than the row's title, as a list under it.
const OPTION_SIZE: f32 = 15.0;
const RADIUS: f32 = 6.0;
/// The free corners of a popup panel, rounder than the row it hangs from.
const PANEL_RADIUS: f32 = 8.0;

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

            ListRow::new(text(label(option)).size(OPTION_SIZE))
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
        // A popup hangs from the row: a hairline marks where the row ends.
        let list: Element<'a, PopupMessage<Message>> = if popup {
            column![
                container(iced::widget::Space::new().height(1))
                    .width(Fill)
                    .style(|theme| {
                        container::Style::default()
                            .background(crate::theme::colors(theme).selection)
                    }),
                list,
            ]
            .into()
        } else {
            list.into()
        };
        let panel = container(list).width(Fill).style(move |theme| {
            if popup {
                let colors = crate::theme::colors(theme);
                container::Style::default()
                    .background(colors.border)
                    .border(Border::default().rounded(iced::border::bottom(PANEL_RADIUS)))
            } else {
                container::Style::default()
            }
        });

        if !popup {
            return Element::new(Selector {
                header,
                panel: panel.into(),
                enabled,
            });
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
    container(text(label).size(13).muted())
        .padding([spacing::XS, spacing::MD])
        .into()
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: String,
    icon: Option<Icon>,
    switch: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut value_row = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        value_row = value_row.push(
            icon.view()
                .width(list_row::BODY_SIZE)
                .height(list_row::BODY_SIZE),
        );
    }

    value_row = value_row.push(text(value).size(list_row::BODY_SIZE).muted());

    let mut row = ListRow::new(
        column![text(title).label().medium(), value_row]
            .width(Fill)
            .spacing(spacing::XS),
    );
    if let Some(switch) = switch {
        row = row.trailing(switch);
    }
    row.into_disclosure_content()
}

/// A selector whose options push the content below down.
struct Selector<'a, Message> {
    header: Element<'a, Message>,
    panel: Element<'a, PopupMessage<Message>>,
    enabled: bool,
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    interaction: Interaction,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.header), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.header);
        tree.children[1].diff(&self.panel);

        if !self.enabled {
            tree.state.downcast_mut::<State>().open = false;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let header = self
            .header
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let header_size = header.size();
        let panel = if tree.state.downcast_ref::<State>().open {
            self.panel
                .as_widget_mut()
                .layout(
                    &mut tree.children[1],
                    renderer,
                    &layout::Limits::new(
                        Size::new(header_size.width, 0.0),
                        Size::new(
                            header_size.width,
                            (limits.max().height - header_size.height).max(0.0),
                        ),
                    ),
                )
                .move_to(Point::new(0.0, header_size.height))
        } else {
            layout::Node::new(Size::ZERO)
        };
        let size = Size::new(header_size.width, header_size.height + panel.size().height);

        layout::Node::with_children(size, vec![header, panel])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let (header, panel) = layout_parts(layout);

        if self.enabled {
            operation.focusable(None, header.bounds(), &mut state.interaction);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.header
                .as_widget_mut()
                .operate(&mut tree.children[0], header, renderer, operation);

            if state.open {
                self.panel.as_widget_mut().operate(
                    &mut tree.children[1],
                    panel,
                    renderer,
                    operation,
                );
            }
        });
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
        let (header_layout, panel_layout) = layout_parts(layout);
        let state = tree.state.downcast_mut::<State>();

        if state.open {
            let mut messages = Vec::new();
            let mut panel_shell = Shell::new(&mut messages);
            self.panel.as_widget_mut().update(
                &mut tree.children[1],
                event,
                panel_layout,
                cursor,
                renderer,
                clipboard,
                &mut panel_shell,
                viewport,
            );

            if popup::forward(shell, panel_shell) {
                state.open = false;
                shell.invalidate_layout();
            }
        }

        self.header.as_widget_mut().update(
            &mut tree.children[0],
            event,
            header_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if state.interaction.update(
            event,
            header_layout
                .bounds()
                .intersection(viewport)
                .unwrap_or_default(),
            cursor,
            self.enabled,
            shell,
        ) {
            state.open = !state.open;
            shell.invalidate_layout();
            shell.request_redraw();
        } else if state.open && dismisses(event, cursor, layout.bounds()) {
            state.open = false;
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let (header, panel) = layout_parts(layout);
        let mut interaction = self.header.as_widget().mouse_interaction(
            &tree.children[0],
            header,
            cursor,
            viewport,
            renderer,
        );

        if state.open {
            interaction = interaction.max(self.panel.as_widget().mouse_interaction(
                &tree.children[1],
                panel,
                cursor,
                viewport,
                renderer,
            ));
        }

        if interaction == mouse::Interaction::default()
            && self.enabled
            && cursor.is_over(header.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            interaction
        }
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
        let state = tree.state.downcast_ref::<State>();
        let (header, panel) = layout_parts(layout);
        let colors = crate::theme::colors(theme);

        if state.open {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border::default().rounded(RADIUS),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(colors.border),
            );
        }

        draw_header_fill(
            renderer,
            theme,
            header.bounds(),
            state
                .interaction
                .state(self.enabled, state.open, header.bounds(), cursor),
            false,
        );
        self.header.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: if self.enabled {
                    colors.text
                } else {
                    colors.muted
                },
            },
            header,
            cursor,
            viewport,
        );
        draw_caret(
            renderer,
            theme,
            disclosure_bounds(header.bounds()),
            f32::from(state.open),
        );

        if state.open {
            self.panel.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                panel,
                cursor,
                viewport,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        y: panel.bounds().y,
                        height: 1.0,
                        ..panel.bounds()
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                Background::Color(colors.selection),
            );
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        let (_, panel) = layout_parts(layout);

        state
            .open
            .then(|| {
                self.panel.as_widget_mut().overlay(
                    &mut tree.children[1],
                    panel,
                    renderer,
                    viewport,
                    translation,
                )
            })?
            .map(|overlay| overlay.map(&PopupMessage::into_inner))
    }
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

        draw_header_fill(
            renderer,
            theme,
            bounds,
            interaction.state(self.enabled, open, bounds, cursor),
            open,
        );
        self.popup
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
        draw_caret(renderer, theme, disclosure_bounds(bounds), f32::from(open));
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

/// Fills the row of a selector, as a list row that is `selected` while its
/// options show. An `attached` panel continues the row, so the row's bottom
/// corners go.
fn draw_header_fill(
    renderer: &mut iced::Renderer,
    theme: &Theme,
    bounds: Rectangle,
    state: ActionState,
    attached: bool,
) {
    let mut style = list_row::action_style(theme, state);
    if attached {
        style.border = Border::default().rounded(iced::border::top(RADIUS));
    }
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: style.border,
            shadow: style.shadow,
            snap: style.snap,
        },
        style
            .background
            .unwrap_or(Background::Color(iced::Color::TRANSPARENT)),
    );
}

fn layout_parts(layout: Layout<'_>) -> (Layout<'_>, Layout<'_>) {
    let mut children = layout.children();
    (
        children.next().expect("selector header"),
        children.next().expect("selector panel"),
    )
}

fn disclosure_bounds(header: Rectangle) -> Rectangle {
    const SIZE: f32 = 20.0;

    Rectangle {
        x: header.x + header.width - list_row::STANDARD_PADDING.right - SIZE,
        y: header.center_y() - SIZE / 2.0,
        width: SIZE,
        height: SIZE,
    }
}

fn dismisses(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor
            .position()
            .is_some_and(|position| !bounds.contains(position)),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            repeat: false,
            ..
        }) => true,
        _ => false,
    }
}
