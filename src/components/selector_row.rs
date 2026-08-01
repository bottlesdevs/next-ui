use iced::{
    Alignment, Background, Border, ContentFit, Element, Event, Fill, Length, Padding, Point,
    Rectangle, Shadow, Size, Theme,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        svg::Renderer as _,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{Space, column, container, row, svg, text},
};

use crate::icons::Icon;

use super::{pressable::event_cursor, spacing, text::TextExt as _};

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
        let options = selector.options;
        let selected = selector
            .selected
            .and_then(|selected| options.iter().position(|option| option == selected));
        let labels: Vec<_> = options.iter().map(selector.label).collect();
        let value = selected
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| selector.placeholder.to_owned());
        let on_selected = selector.on_selected.map(|on_selected| {
            Box::new(move |index: usize| on_selected(options[index].clone()))
                as Box<dyn Fn(usize) -> Message>
        });
        let children = std::iter::once(header(selector.title, &value, selector.icon))
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
        })
    }
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: &str,
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

    value_row = value_row.push(text(value.to_owned()).detail().muted());

    container(
        row![
            column![text(title).label(), value_row]
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
}

impl<Message> Selector<'_, Message> {
    fn option_count(&self) -> usize {
        self.children.len().saturating_sub(1)
    }

    fn is_enabled(&self) -> bool {
        self.on_selected.is_some() && self.option_count() > 0
    }
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    highlighted: Option<usize>,
    hovered: Option<usize>,
    pressed: Option<Pressed>,
    focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pressed {
    Header,
    Option(usize),
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.pressed = None;
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<State>();

        let option_count = self.option_count();

        if !self.is_enabled() {
            state.open = false;
            state.highlighted = None;
            state.hovered = None;
            state.pressed = None;
        } else if state.highlighted.is_none_or(|index| index >= option_count) {
            state.highlighted = self.selected.or(Some(0));
        }

        if state.hovered.is_some_and(|index| index >= option_count) {
            state.hovered = None;
        }

        if matches!(state.pressed, Some(Pressed::Option(index)) if index >= option_count) {
            state.pressed = None;
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
        let open = tree.state.downcast_ref::<State>().open;
        let option_panel = if open {
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
        let height = header_height + option_panel.size().height;

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
            operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<State>());
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
        if !self.is_enabled() {
            return;
        }

        let (header, options) = layout_parts(layout);
        let state = tree.state.downcast_mut::<State>();
        let was_open = state.open;
        let was_highlighted = state.highlighted;
        let was_hovered = state.hovered;
        let target = hit_target(state.open, header, &options, event_cursor(event, cursor));
        state.hovered = match target {
            Some(Pressed::Option(index)) => Some(index),
            _ => None,
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                state.pressed = target;
                state.focused = target.is_some();

                if target.is_some() {
                    shell.capture_event();
                } else if state.open {
                    state.open = false;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                let pressed = state.pressed.take();

                if pressed == target {
                    match target {
                        Some(Pressed::Header) => {
                            state.open = !state.open;
                            state.highlighted = self.selected.or(Some(0));
                            shell.capture_event();
                        }
                        Some(Pressed::Option(index)) => {
                            if let Some(on_selected) = &self.on_selected {
                                shell.publish(on_selected(index));
                            }
                            state.open = false;
                            state.highlighted = Some(index);
                            shell.capture_event();
                        }
                        _ => {}
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                state.hovered = None;
                state.pressed = None;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) if state.focused => {
                let last = self.option_count().checked_sub(1);

                match key.as_ref() {
                    keyboard::Key::Named(key::Named::ArrowDown) => {
                        if let Some(last) = last {
                            state.highlighted = Some(if state.open {
                                state
                                    .highlighted
                                    .map_or(self.selected.unwrap_or(0), |index| {
                                        (index + 1).min(last)
                                    })
                            } else {
                                self.selected.unwrap_or(0)
                            });
                            state.open = true;
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::ArrowUp) => {
                        if let Some(last) = last {
                            state.highlighted = Some(if state.open {
                                state
                                    .highlighted
                                    .unwrap_or_else(|| self.selected.unwrap_or(0))
                                    .saturating_sub(1)
                            } else {
                                self.selected.unwrap_or(last)
                            });
                            state.open = true;
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::Home) if last.is_some() => {
                        state.open = true;
                        state.highlighted = Some(0);
                        shell.capture_event();
                    }
                    keyboard::Key::Named(key::Named::End) => {
                        if let Some(last) = last {
                            state.open = true;
                            state.highlighted = Some(last);
                            shell.capture_event();
                        }
                    }
                    keyboard::Key::Named(key::Named::Enter | key::Named::Space) => {
                        if state.open {
                            if let Some(index) = state.highlighted {
                                if let Some(on_selected) = &self.on_selected {
                                    shell.publish(on_selected(index));
                                }
                                state.open = false;
                            }
                        } else {
                            state.open = true;
                            state.highlighted = self.selected.or(Some(0));
                        }

                        shell.capture_event();
                    }
                    keyboard::Key::Named(key::Named::Escape) if state.open => {
                        state.open = false;
                        shell.capture_event();
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if !state.open {
            state.hovered = None;
        }

        if state.open != was_open {
            shell.invalidate_layout();
        }

        if state.open != was_open
            || state.highlighted != was_highlighted
            || state.hovered != was_hovered
        {
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

        if self.is_enabled() && hit_target(state.open, header, &options, cursor).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let hovered = self.is_enabled() && cursor.is_over(bounds);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default().rounded(8),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(if state.open || hovered {
                theme.extended_palette().background.neutral.color
            } else {
                theme.extended_palette().background.weak.color
            }),
        );

        let (header, options) = layout_parts(layout);
        self.children[0].as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            header,
            cursor,
            viewport,
        );
        let header_content = header.children().next().expect("selector header content");
        let caret_slot = header_content
            .children()
            .nth(1)
            .expect("selector caret slot");
        draw_caret(renderer, caret_slot.bounds(), state.open);

        if state.open {
            let highlighted = options
                .iter()
                .position(|layout| cursor.is_over(layout.bounds()))
                .or(state.hovered)
                .or(state.highlighted);

            let line = Rectangle {
                y: header.bounds().y + header.bounds().height,
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
                let is_highlighted = highlighted == Some(index);

                if is_highlighted {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: option_layout.bounds(),
                            border: Border::default().rounded(8),
                            shadow: Shadow::default(),
                            snap: true,
                        },
                        Background::Color(theme.extended_palette().background.stronger.color),
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
                    viewport,
                );
            }
        }

        if !self.is_enabled() {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border::default().rounded(8),
                    shadow: Shadow::default(),
                    snap: true,
                },
                crate::theme::SCRIM,
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

fn draw_caret(renderer: &mut iced::Renderer, slot: Rectangle, open: bool) {
    let handle = Icon::DownCaret.handle();
    let Size { width, height } = renderer.measure_svg(&handle);
    let size = ContentFit::Contain.fit(Size::new(width as f32, height as f32), slot.size());
    let bounds = Rectangle::new(
        Point::new(
            slot.center_x() - size.width / 2.0,
            slot.center_y() - size.height / 2.0,
        ),
        size,
    );

    renderer.draw_svg(
        iced::advanced::svg::Svg {
            handle,
            color: None,
            rotation: (if open { std::f32::consts::PI } else { 0.0 }).into(),
            opacity: 1.0,
        },
        bounds,
        slot,
    );
}

fn hit_target(
    open: bool,
    header: Layout<'_>,
    options: &[Layout<'_>],
    cursor: mouse::Cursor,
) -> Option<Pressed> {
    if cursor.is_over(header.bounds()) {
        Some(Pressed::Header)
    } else if open {
        options
            .iter()
            .position(|layout| cursor.is_over(layout.bounds()))
            .map(Pressed::Option)
    } else {
        None
    }
}
