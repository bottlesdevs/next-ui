use iced::{
    Alignment, Background, Border, ContentFit, Element, Event, Fill, Length, Padding, Point,
    Rectangle, Shadow, Size, Theme,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    animation::{Animation, Easing},
    keyboard::{self, key},
    time::Instant,
    touch,
    widget::{Space, column, container, row, svg, text},
    window,
};

use crate::icons::Icon;

use super::{
    control::{Interaction, Outcome},
    draw_caret, list_row, reconcile_index, spacing,
    text::TextExt as _,
};

const OPTION_PANEL_PADDING: Padding = Padding {
    top: spacing::MD,
    right: spacing::SM,
    bottom: spacing::MD,
    left: spacing::SM,
};

pub struct SelectorRow<'a, T, Message> {
    title: &'a str,
    options: &'a [T],
    selected: Option<&'a T>,
    on_selected: Option<Box<dyn Fn(T) -> Message + 'a>>,
    placeholder: &'a str,
    label: Box<dyn Fn(&T) -> String + 'a>,
    key: Option<Box<dyn Fn(&T) -> String + 'a>>,
    icon: Option<Icon>,
}

impl<'a, T: ToString, Message> SelectorRow<'a, T, Message> {
    pub fn new(title: &'a str, options: &'a [T], selected: Option<&'a T>) -> Self {
        Self {
            title,
            options,
            selected,
            on_selected: None,
            placeholder: "",
            label: Box::new(ToString::to_string),
            key: None,
            icon: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn label(mut self, label: impl Fn(&T) -> String + 'a) -> Self {
        self.label = Box::new(label);
        self
    }

    pub fn key(mut self, key: impl Fn(&T) -> String + 'a) -> Self {
        self.key = Some(Box::new(key));
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

    pub fn on_selected_maybe(mut self, on_selected: Option<impl Fn(T) -> Message + 'a>) -> Self {
        self.on_selected = on_selected.map(|on_selected| Box::new(on_selected) as _);
        self
    }
}

impl<'a, T, Message> From<SelectorRow<'a, T, Message>> for Element<'a, Message>
where
    T: Clone + PartialEq + ToString + 'a,
    Message: 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let SelectorRow {
            title,
            options,
            selected,
            on_selected,
            placeholder,
            label,
            key,
            icon,
        } = selector;
        let selected =
            selected.and_then(|selected| options.iter().position(|option| option == selected));
        let labels: Vec<_> = options.iter().map(label).collect();
        let keys = key.map_or_else(|| labels.clone(), |key| options.iter().map(key).collect());
        let value = selected
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| placeholder.to_owned());
        let on_selected = on_selected.map(|on_selected| {
            Box::new(move |index: usize| on_selected(options[index].clone()))
                as Box<dyn Fn(usize) -> Message>
        });
        let children = std::iter::once(header(title, value, icon))
            .chain(labels.into_iter().map(|label| {
                container(text(label).label())
                    .width(Fill)
                    .padding([spacing::SM, spacing::SM])
                    .into()
            }))
            .collect();

        Element::new(Selector {
            children,
            on_selected,
            selected,
            keys,
        })
    }
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: String,
    icon: Option<Icon>,
) -> Element<'a, Message> {
    let mut value_row = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        value_row = value_row.push(
            svg(icon.handle())
                .width(16)
                .height(16)
                .content_fit(ContentFit::Contain),
        );
    }

    value_row = value_row.push(text(value).detail().muted());

    container(
        row![
            column![text(title).label().medium(), value_row]
                .width(Fill)
                .spacing(spacing::XS),
            Space::new().width(20).height(20),
        ]
        .align_y(Alignment::Center)
        .spacing(spacing::MD),
    )
    .width(Fill)
    .padding([spacing::SM, spacing::LG])
    .into()
}

struct Selector<'a, Message> {
    children: Vec<Element<'a, Message>>,
    on_selected: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    selected: Option<usize>,
    keys: Vec<String>,
}

impl<Message> Selector<'_, Message> {
    fn option_count(&self) -> usize {
        self.children.len().saturating_sub(1)
    }

    fn is_enabled(&self) -> bool {
        self.on_selected.is_some() && self.option_count() > 0
    }
}

#[derive(Debug)]
struct State {
    expansion: Animation<bool>,
    highlighted: Option<usize>,
    header: Interaction,
    options: Vec<Interaction>,
    keys: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expansion: Animation::new(false).very_quick().easing(Easing::EaseOut),
            highlighted: None,
            header: Interaction::default(),
            options: Vec::new(),
            keys: Vec::new(),
        }
    }
}

impl State {
    fn is_open(&self) -> bool {
        self.expansion.value()
    }

    fn set_open(&mut self, open: bool, now: Instant) {
        if self.is_open() != open {
            self.expansion.go_mut(open, now);

            if !open {
                for option in &mut self.options {
                    *option = Interaction::default();
                }
            }
        }
    }

    fn expansion(&self, now: Instant) -> f32 {
        self.expansion.interpolate(0.0, 1.0, now)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let mut state = State::default();
        state.options = (0..self.option_count())
            .map(|_| Interaction::default())
            .collect();
        state.keys.clone_from(&self.keys);

        if self.is_enabled() {
            state.highlighted = self.selected.or(Some(0));
        }

        tree::State::new(state)
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<State>();
        let option_count = self.option_count();
        let highlighted = reconcile_index(&state.keys, state.highlighted, &self.keys);

        if state.keys != self.keys {
            state.options = (0..option_count).map(|_| Interaction::default()).collect();
        } else {
            state
                .options
                .resize_with(option_count, Interaction::default);
        }

        state.keys.clone_from(&self.keys);

        if !self.is_enabled() {
            state.set_open(false, Instant::now());
            state.highlighted = None;
            state.header = Interaction::default();
        } else {
            state.highlighted = highlighted.or(self.selected).or(Some(0));
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
        let (header, options) = self.children.split_first_mut().expect("selector header");
        let (header_tree, option_trees) = tree.children.split_first_mut().expect("selector header");
        let header = header
            .as_widget_mut()
            .layout(header_tree, renderer, &limits);
        let header_height = header.size().height;
        let width = header.size().width;
        let expansion = tree.state.downcast_ref::<State>().expansion(Instant::now());
        let option_panel = if expansion > 0.0 {
            layout::flex::resolve(
                layout::flex::Axis::Vertical,
                renderer,
                &limits.shrink(Size::new(0.0, header_height)),
                Length::Fill,
                Length::Shrink,
                OPTION_PANEL_PADDING,
                0.0,
                Alignment::Start,
                options,
                option_trees,
            )
            .move_to(Point::new(0.0, header_height))
        } else {
            layout::Node::new(Size::ZERO)
        };
        let height = header_height + option_panel.size().height * expansion;

        layout::Node::with_children(Size::new(width, height), vec![header, option_panel])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.is_enabled() {
            let header = layout.children().next().expect("selector header");
            operation.focusable(
                None,
                header.bounds(),
                &mut tree.state.downcast_mut::<State>().header,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && tree
                .state
                .downcast_ref::<State>()
                .expansion
                .is_animating(*now)
        {
            shell.invalidate_layout();
            shell.request_redraw();
        }

        if !self.is_enabled() {
            return;
        }

        let (header, options) = layout_parts(layout);
        let state = tree.state.downcast_mut::<State>();
        let was_open = state.is_open();
        let was_highlighted = state.highlighted;

        if !matches!(event, Event::Keyboard(_)) {
            let mut pointer_captured = false;
            let header_outcome =
                state
                    .header
                    .update(event, header.bounds(), cursor, true, true, false, shell);
            pointer_captured |= header_outcome != Outcome::Ignored;

            let mut selected = None;

            if state.is_open() {
                for (index, (interaction, option)) in
                    state.options.iter_mut().zip(&options).enumerate()
                {
                    let outcome = interaction.update(
                        event,
                        option.bounds(),
                        cursor,
                        true,
                        true,
                        false,
                        shell,
                    );
                    pointer_captured |= outcome != Outcome::Ignored;

                    if outcome == Outcome::Activated {
                        selected = Some(index);
                    }
                }
            }

            if matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. })
            ) {
                if pointer_captured {
                    operation::Focusable::focus(&mut state.header);
                } else if state.is_open() {
                    state.set_open(false, Instant::now());
                }
            }

            if header_outcome == Outcome::Activated {
                state.set_open(!state.is_open(), Instant::now());
                state.highlighted = self.selected.or(Some(0));
            } else if let Some(index) = selected {
                if let Some(on_selected) = &self.on_selected {
                    shell.publish(on_selected(index));
                }
                state.set_open(false, Instant::now());
                state.highlighted = Some(index);
            }
        }

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) if operation::Focusable::is_focused(&state.header) => {
                let last = self.option_count().checked_sub(1);

                match key.as_ref() {
                    keyboard::Key::Named(key::Named::ArrowDown) => {
                        if let Some(last) = last {
                            state.highlighted = Some(if state.is_open() {
                                state
                                    .highlighted
                                    .map_or(self.selected.unwrap_or(0), |index| {
                                        (index + 1).min(last)
                                    })
                            } else {
                                self.selected.unwrap_or(0)
                            });
                            state.set_open(true, Instant::now());
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::ArrowUp) => {
                        if let Some(last) = last {
                            state.highlighted = Some(if state.is_open() {
                                state
                                    .highlighted
                                    .unwrap_or_else(|| self.selected.unwrap_or(0))
                                    .saturating_sub(1)
                            } else {
                                self.selected.unwrap_or(last)
                            });
                            state.set_open(true, Instant::now());
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::Home) if last.is_some() => {
                        state.set_open(true, Instant::now());
                        state.highlighted = Some(0);
                        shell.capture_event();
                    }
                    keyboard::Key::Named(key::Named::End) => {
                        if let Some(last) = last {
                            state.set_open(true, Instant::now());
                            state.highlighted = Some(last);
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::Enter | key::Named::Space) => {
                        if state.is_open() {
                            if let Some(index) = state.highlighted {
                                if let Some(on_selected) = &self.on_selected {
                                    shell.publish(on_selected(index));
                                }
                                state.set_open(false, Instant::now());
                            }
                        } else {
                            state.set_open(true, Instant::now());
                            state.highlighted = self.selected.or(Some(0));
                        }

                        shell.capture_event();
                    }
                    keyboard::Key::Named(key::Named::Escape) if state.is_open() => {
                        state.set_open(false, Instant::now());
                        shell.capture_event();
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if state.is_open() != was_open {
            shell.invalidate_layout();
        }

        if state.is_open() != was_open || state.highlighted != was_highlighted {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let (header, options) = layout_parts(layout);

        if !self.is_enabled() {
            return mouse::Interaction::default();
        }

        let header_interaction =
            state
                .header
                .mouse_interaction(true, true, header.bounds(), cursor);

        options
            .into_iter()
            .zip(&state.options)
            .filter(|_| state.is_open())
            .fold(header_interaction, |current, (option, interaction)| {
                current.max(interaction.mouse_interaction(true, true, option.bounds(), cursor))
            })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let expansion = state.expansion(Instant::now());
        let mut control_state = state.header.state(self.is_enabled(), true, bounds, cursor);
        control_state.expanded = expansion > 0.0;
        let style = list_row::style(theme, control_state);

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

        let (header, options) = layout_parts(layout);
        self.children[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color,
            },
            header,
            cursor,
            viewport,
        );
        let header_content = header.children().next().expect("selector header content");
        let caret_slot = header_content
            .children()
            .nth(1)
            .expect("selector caret slot");
        draw_caret(renderer, caret_slot.bounds(), expansion);

        if expansion > 0.0 {
            let panel = Rectangle {
                x: bounds.x,
                y: header.bounds().y + header.bounds().height,
                width: bounds.width,
                height: (bounds.height - header.bounds().height).max(0.0),
            };

            if let Some(clip) = panel.intersection(viewport) {
                renderer.with_layer(clip, |renderer| {
                    let hovered =
                        options
                            .iter()
                            .zip(&state.options)
                            .position(|(option, interaction)| {
                                interaction
                                    .state(true, true, option.bounds(), cursor)
                                    .hovered
                            });
                    let highlighted = hovered.or(state.highlighted);
                    let line = Rectangle {
                        y: panel.y,
                        height: 1.0,
                        ..bounds
                    };

                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: line,
                            border: Border::default(),
                            shadow: Shadow::default(),
                            snap: true,
                        },
                        theme.extended_palette().background.stronger.color,
                    );

                    for (index, option_layout) in options.into_iter().enumerate() {
                        let mut option_state =
                            state.options[index].state(true, true, option_layout.bounds(), cursor);
                        option_state.keyboard_highlighted =
                            hovered.is_none() && highlighted == Some(index);
                        let is_highlighted =
                            option_state.hovered || option_state.keyboard_highlighted;

                        if is_highlighted {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: option_layout.bounds(),
                                    border: Border::default().rounded(6),
                                    shadow: Shadow::default(),
                                    snap: true,
                                },
                                Background::Color(
                                    theme.extended_palette().background.stronger.color,
                                ),
                            );
                        }

                        self.children[index + 1].as_widget().draw(
                            &tree.children[index + 1],
                            renderer,
                            theme,
                            &renderer::Style {
                                text_color: if is_highlighted {
                                    theme.palette().text
                                } else {
                                    theme.extended_palette().secondary.base.text
                                },
                            },
                            option_layout,
                            cursor,
                            &clip,
                        );
                    }
                });
            }
        }

        if let Some(foreground) = style.foreground {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: Shadow::default(),
                    snap: style.snap,
                },
                foreground,
            );
        }
    }
}

impl<'a, Message: 'a> From<Selector<'a, Message>> for Element<'a, Message> {
    fn from(selector: Selector<'a, Message>) -> Self {
        Element::new(selector)
    }
}

fn layout_parts<'a>(layout: Layout<'a>) -> (Layout<'a>, Vec<Layout<'a>>) {
    let mut children = layout.children();
    let header = children.next().expect("selector header");
    let options = children
        .next()
        .expect("selector option panel")
        .children()
        .collect();

    (header, options)
}
