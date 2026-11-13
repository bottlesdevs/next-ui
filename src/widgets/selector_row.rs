use iced::{
    Alignment, Background, Border, Element, Event, Fill, Length, Point, Rectangle, Shadow, Size,
    Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{Column, column, container, row, scrollable, text},
};

use crate::icons::Icon;

use super::{
    anchored_overlay::{
        DismissOutcome, DismissReason, KeepOpen, PopupCore, PopupFocus, PopupFocusState, kept_open,
    },
    button::{Button, ButtonKind},
    draw_caret,
    focusable_action::Interaction,
    list_row::{self, ListRow},
    menu::{self, item as menu_item, row_content},
    progress_bar::ProgressBar,
    spacing,
    switcher::Switcher,
    text::TextExt as _,
};

const PANEL_PADDING: [f32; 2] = [spacing::MD, spacing::SM];
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
    available: Vec<AvailableOption<'a, Message>>,
    popup: bool,
    switch: Option<Switcher<'a, Message>>,
    downloads: bool,
}

/// An option offered for download in a [`SelectorRow`], listed under the
/// options that are ready to select.
pub struct AvailableOption<'a, Message> {
    label: text::Fragment<'a>,
    state: AvailableState<Message>,
}

enum AvailableState<Message> {
    Download(Message),
    Progress(f32),
}

impl<'a, Message> AvailableOption<'a, Message> {
    /// An option with a Download button that publishes `on_download`.
    pub fn download(label: impl text::IntoFragment<'a>, on_download: Message) -> Self {
        Self {
            label: label.into_fragment(),
            state: AvailableState::Download(on_download),
        }
    }

    /// An option that is downloading, with its progress.
    pub fn progress(label: impl text::IntoFragment<'a>, progress: f32) -> Self {
        Self {
            label: label.into_fragment(),
            state: AvailableState::Progress(progress),
        }
    }
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
            available: Vec::new(),
            popup: false,
            switch: None,
            downloads: false,
        }
    }

    /// Lists `available` options under the selectable ones, e.g. releases
    /// that can be downloaded. The two groups get "Downloaded" and
    /// "Available" headings.
    pub fn available(
        mut self,
        available: impl IntoIterator<Item = AvailableOption<'a, Message>>,
    ) -> Self {
        self.downloads = true;
        self.available = available.into_iter().collect();
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
        // Releases to download are worth opening for even with none to pick.
        let enabled = selector.on_selected.is_some()
            && !(selector.options.is_empty() && selector.available.is_empty());
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
                .map(|on_selected| on_selected(option.clone()));

            let current = selected == Some(option);
            let mut content = row_content(label(option).into(), None, None);
            if current {
                content = content.push(Icon::Checkmark.view().width(16).height(16));
            }
            menu_item(content, message, current).into()
        });
        let mut list = Column::new().width(Fill);
        // A selector that can download names what's already downloaded,
        // whether or not anything is on offer right now, unless that's
        // nothing.
        if selector.downloads && !selector.options.is_empty() {
            list = list.push(section_heading("Downloaded"));
        }
        list = list.extend(rows);
        if !selector.available.is_empty() {
            if !selector.options.is_empty() {
                list = list.push(menu::separator());
            }
            list = list
                .push(section_heading("Available"))
                .extend(selector.available.into_iter().map(available_row));
        }
        let popup = selector.popup;
        let list = scrollable(container(list).width(Fill).padding(PANEL_PADDING))
            .width(Fill)
            .style(crate::theme::scrollbar);
        // A popup hangs from the row: a hairline marks where the row ends.
        let list: Element<'a, Message> = if popup {
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
                    // Offset by as much as it blurs, so the shadow fades out
                    // before the row: the popup continues the row rather
                    // than floating over it.
                    .shadow(Shadow {
                        color: iced::Color::BLACK.scale_alpha(0.35),
                        offset: Vector::new(0.0, 16.0),
                        blur_radius: 16.0,
                    })
            } else {
                container::Style::default()
            }
        });

        Element::new(Selector {
            header,
            panel: panel.into(),
            enabled,
            popup,
        })
    }
}

fn section_heading<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(14).muted())
        .padding([spacing::XS, spacing::MD])
        .into()
}

fn available_row<'a, Message: Clone + 'a>(
    option: AvailableOption<'a, Message>,
) -> Element<'a, Message> {
    let body: Element<'a, Message> = match option.state {
        AvailableState::Download(message) => row![
            text(option.label).label().muted().width(Fill),
            // Downloading keeps the options open, so the download's progress
            // shows in place.
            KeepOpen::new(
                Button::new("Download")
                    .trailing_icon(Icon::Download)
                    .icon_size(16.0)
                    .kind(ButtonKind::Secondary)
                    .on_press(message),
            ),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into(),
        AvailableState::Progress(progress) => column![
            text(option.label).label().muted(),
            ProgressBar::new(progress).percentage(),
        ]
        .spacing(4)
        .into(),
    };

    container(body).padding([spacing::XS, spacing::MD]).into()
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

struct Selector<'a, Message> {
    header: Element<'a, Message>,
    panel: Element<'a, Message>,
    enabled: bool,
    popup: bool,
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    interaction: Interaction,
    panel_focus: PopupFocusState,
    restore_focus: bool,
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
        tree.diff_children(&[&self.header, &self.panel]);

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
        let panel = if !self.popup && tree.state.downcast_ref::<State>().open {
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

            if state.open && !self.popup {
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

        if state.open && !self.popup {
            let mut messages = Vec::new();
            let mut panel_shell = Shell::new(&mut messages);
            let keep_open = kept_open(|| {
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
            });

            if !panel_shell.is_empty() && !keep_open {
                state.open = false;
                panel_shell.invalidate_layout();
            }
            shell.merge(panel_shell, std::convert::identity);
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
        } else if state.open && !self.popup && dismisses(event, cursor, layout.bounds()) {
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

        if state.open && !self.popup {
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

        if state.open && !self.popup {
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

        let mut header_state =
            state
                .interaction
                .state(self.enabled, false, header.bounds(), cursor);
        header_state.selected = state.open;
        let mut header_style = list_row::action_style(theme, header_state);
        // An open popup continues the row, so the row's bottom corners go.
        if state.open && self.popup {
            header_style.border = Border::default().rounded(iced::border::top(RADIUS));
        }
        renderer.fill_quad(
            renderer::Quad {
                bounds: header.bounds(),
                border: header_style.border,
                shadow: header_style.shadow,
                snap: header_style.snap,
            },
            header_style
                .background
                .unwrap_or(Background::Color(iced::Color::TRANSPARENT)),
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

        if state.open && !self.popup {
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
        if self.popup {
            let (header, _) = layout_parts(layout);
            let (state, children) = (&mut tree.state, &mut tree.children);
            let State {
                open,
                panel_focus,
                restore_focus,
                ..
            } = state.downcast_mut::<State>();

            if !*open {
                return None;
            }

            let anchor = Rectangle::new(
                header.bounds().position() + translation,
                header.bounds().size(),
            );

            return Some(overlay::Element::new(Box::new(
                PopupCore::new(
                    anchor,
                    *viewport,
                    self.panel.as_widget_mut(),
                    &mut children[1],
                    0.0,
                    PopupFocus {
                        state: panel_focus,
                        restore_anchor: restore_focus,
                        targets: &[],
                    },
                    move |reason| {
                        *open = false;
                        match reason {
                            DismissReason::FocusLost | DismissReason::OutsidePress => {
                                DismissOutcome::PassThrough
                            }
                            DismissReason::ContentMessage { keyboard: false } => {
                                DismissOutcome::Captured
                            }
                            DismissReason::Escape
                            | DismissReason::ContentMessage { keyboard: true } => {
                                DismissOutcome::CapturedAndRestoreFocus
                            }
                        }
                    },
                )
                .match_anchor_width()
                .attached(),
            )));
        }

        let state = tree.state.downcast_ref::<State>();
        let (_, panel) = layout_parts(layout);

        state.open.then(|| {
            self.panel.as_widget_mut().overlay(
                &mut tree.children[1],
                panel,
                renderer,
                viewport,
                translation,
            )
        })?
    }
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
