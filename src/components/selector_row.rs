use iced::{
    Alignment, Background, Border, ContentFit, Element, Event, Fill, Length, Padding, Point,
    Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    touch,
    widget::{button, column, container, row, svg, text},
};

use crate::icons::Icon;

use super::{
    pressable::{Pressable, SharedFlag, Status},
    text::TextExt as _,
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
    Message: Clone + 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let selected = selector.selected.and_then(|selected| {
            selector
                .options
                .iter()
                .position(|option| option == selected)
        });
        let labels: Vec<_> = selector
            .options
            .iter()
            .map(|option| (selector.label)(option))
            .collect();
        let value = selected
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| selector.placeholder.to_owned());
        let messages: Vec<_> = selector.on_selected.map_or_else(Vec::new, |on_selected| {
            selector.options.iter().cloned().map(on_selected).collect()
        });
        let highlights: Vec<_> = labels.iter().map(|_| SharedFlag::default()).collect();
        let children = [false, true]
            .into_iter()
            .map(|expanded| header(selector.title, &value, selector.icon, expanded))
            .chain(labels.iter().zip(&messages).zip(&highlights).map(
                |((label, message), highlighted)| {
                    let highlighted = highlighted.clone();

                    Pressable::new(text(label.clone()).label())
                        .width(Fill)
                        .padding(Padding::from([10, 9]))
                        .focusable(false)
                        .on_press(message.clone())
                        .style(move |theme, status| option_style(theme, status, highlighted.get()))
                        .into()
                },
            ))
            .collect();

        Element::new(Selector {
            children,
            messages,
            highlights,
            selected,
        })
    }
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: &str,
    icon: Option<Icon>,
    expanded: bool,
) -> Element<'a, Message> {
    let mut value_row = row![].spacing(12).align_y(Alignment::Center);

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
                .spacing(8),
            svg(Icon::DownCaret.handle())
                .width(20)
                .height(20)
                .content_fit(ContentFit::Contain)
                .rotation(if expanded { std::f32::consts::PI } else { 0.0 }),
        ]
        .align_y(Alignment::Center)
        .spacing(16),
    )
    .width(Fill)
    .padding(Padding::from([14, 24]))
    .into()
}

struct Selector<'a, Message> {
    children: Vec<Element<'a, Message>>,
    messages: Vec<Message>,
    highlights: Vec<SharedFlag>,
    selected: Option<usize>,
}

impl<Message> Selector<'_, Message> {
    fn is_enabled(&self) -> bool {
        !self.messages.is_empty()
    }
}

#[derive(Debug, Default)]
struct State {
    open: bool,
    highlighted: Option<usize>,
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

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
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

        if self.messages.is_empty() {
            state.open = false;
            state.highlighted = None;
        } else if state
            .highlighted
            .is_none_or(|index| index >= self.messages.len())
        {
            state.highlighted = self.selected.or(Some(0));
        }

        if matches!(state.pressed, Some(Pressed::Option(index)) if index >= self.messages.len()) {
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
        let mut children = Vec::with_capacity(self.children.len());
        let collapsed =
            self.children[0]
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits);
        let expanded =
            self.children[1]
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &limits);
        let header_height = collapsed.size().height.max(expanded.size().height);
        let width = collapsed.size().width.max(expanded.size().width);
        children.push(collapsed);
        children.push(expanded);

        let open = tree.state.downcast_ref::<State>().open;
        let mut y = header_height + if open { 17.0 } else { 0.0 };

        for (index, child) in self.children[2..].iter_mut().enumerate() {
            let node = child.as_widget_mut().layout(
                &mut tree.children[index + 2],
                renderer,
                &limits.shrink(Size::new(20.0, 0.0)),
            );
            let height = node.size().height;
            children.push(node.move_to(Point::new(10.0, y)));

            if open {
                y += height;
            }
        }

        layout::Node::with_children(
            Size::new(width, if open { y + 16.0 } else { header_height }),
            children,
        )
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
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if !self.is_enabled() {
            return;
        }

        let open = tree.state.downcast_ref::<State>().open;
        let mut children = layout.children();
        let collapsed = children.next().expect("collapsed selector header");
        let expanded = children.next().expect("expanded selector header");
        let header = if open { expanded } else { collapsed };
        let options: Vec<_> = children.collect();

        if open {
            for (index, option_layout) in options.iter().enumerate() {
                self.children[index + 2].as_widget_mut().update(
                    &mut tree.children[index + 2],
                    event,
                    *option_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }

        let child_captured = shell.is_event_captured();
        let state = tree.state.downcast_mut::<State>();
        let was_open = state.open;
        let was_highlighted = state.highlighted;
        let target = hit_target(state.open, header, &options, event_cursor(event, cursor));

        if child_captured {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) => {
                    state.pressed = target;
                    state.focused = target.is_some();
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. }) => {
                    let pressed = state.pressed.take();

                    if pressed == target
                        && let Some(Pressed::Option(index)) = target
                    {
                        state.open = false;
                        state.highlighted = Some(index);
                    }
                }
                Event::Touch(touch::Event::FingerLost { .. }) => state.pressed = None,
                _ => {}
            }

            if state.open != was_open {
                shell.invalidate_layout();
            }

            if state.open != was_open || state.highlighted != was_highlighted {
                shell.request_redraw();
            }

            return;
        }

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
                        Some(Pressed::Header) if !self.messages.is_empty() => {
                            state.open = !state.open;
                            state.highlighted = self.selected.or(Some(0));
                            shell.capture_event();
                        }
                        Some(Pressed::Option(index)) => {
                            shell.publish(self.messages[index].clone());
                            state.open = false;
                            state.highlighted = Some(index);
                            shell.capture_event();
                        }
                        _ => {}
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => state.pressed = None,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) if state.focused => {
                let last = self.messages.len().checked_sub(1);

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
                                shell.publish(self.messages[index].clone());
                                state.open = false;
                            }
                        } else if !self.messages.is_empty() {
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

        if state.open != was_open {
            shell.invalidate_layout();
        }

        if state.open != was_open || state.highlighted != was_highlighted {
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
        let mut children = layout.children();
        let collapsed = children.next().expect("collapsed selector header");
        let expanded = children.next().expect("expanded selector header");
        let header = if state.open { expanded } else { collapsed };
        let options: Vec<_> = children.collect();

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

        let mut children = layout.children();
        let collapsed = children.next().expect("collapsed selector header");
        let expanded = children.next().expect("expanded selector header");
        let header = if state.open { expanded } else { collapsed };
        let options: Vec<_> = children.collect();
        self.children[usize::from(state.open)].as_widget().draw(
            &tree.children[usize::from(state.open)],
            renderer,
            theme,
            renderer_style,
            header,
            cursor,
            viewport,
        );

        if state.open {
            let highlighted = options
                .iter()
                .position(|layout| cursor.is_over(layout.bounds()))
                .or(state.highlighted);

            for (index, option) in self.highlights.iter().enumerate() {
                option.set(highlighted == Some(index));
            }

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
                self.children[index + 2].as_widget().draw(
                    &tree.children[index + 2],
                    renderer,
                    theme,
                    renderer_style,
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

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        None
    }
}

fn option_style(theme: &Theme, status: Status, highlighted: bool) -> button::Style {
    let highlighted =
        highlighted || matches!(status, Status::Hovered | Status::Pressed | Status::Focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.base.text
        },
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

impl<'a, Message: Clone + 'a> From<Selector<'a, Message>> for Element<'a, Message> {
    fn from(selector: Selector<'a, Message>) -> Self {
        Element::new(selector)
    }
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

fn event_cursor(event: &Event, cursor: mouse::Cursor) -> mouse::Cursor {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => mouse::Cursor::Available(*position),
        _ => cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::{Pressed, SelectorRow, State, Status, option_style};

    #[test]
    fn selection_action_controls_availability() {
        let options = ["One"];
        let disabled = SelectorRow::<_, ()>::new("Selector", &options, None);
        let enabled = SelectorRow::new("Selector", &options, None).on_selected(|_| ());

        assert!(disabled.on_selected.is_none());
        assert!(enabled.on_selected.is_some());
    }

    #[test]
    fn changing_options_clamps_the_keyboard_highlight() {
        let mut state = State {
            open: true,
            highlighted: Some(4),
            pressed: Some(Pressed::Option(4)),
            focused: true,
        };
        let option_count = 2;

        if state.highlighted.is_none_or(|index| index >= option_count) {
            state.highlighted = Some(0);
        }

        assert_eq!(state.highlighted, Some(0));
    }

    #[test]
    fn hovered_options_use_the_highlight_style() {
        let theme = crate::theme::theme();

        assert!(
            option_style(&theme, Status::Hovered, false)
                .background
                .is_some()
        );
        assert!(
            option_style(&theme, Status::Active, false)
                .background
                .is_none()
        );
    }
}
