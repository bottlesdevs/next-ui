use iced::{
    Background, Element, Event, Fill, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    alignment::Vertical,
    keyboard::{self, key},
    touch,
    widget::{
        Space, column, container, row, rule, scrollable, svg, text, text::Fragment,
        text::IntoFragment,
    },
    window,
};

use crate::{icons::Icon, theme};

use super::{
    button::{Button, ButtonKind},
    control::focus_first_descendant,
    focusable_action::Interaction,
    spacing,
    text::TextExt as _,
};

const INITIAL_LOG_RATIO: f32 = 0.3;
const RESIZE_TARGET_HEIGHT: f32 = 8.0;
const UNBOUNDED_LOG_HEIGHT: f32 = 180.0;
const ICON_SIZE: f32 = 16.0;
const TEXT_SIZE: f32 = 12.0;

/// Domain-neutral status presentation.
pub struct StatusBar<'a> {
    status: Fragment<'a>,
    status_icon: Icon,
    details: Vec<(Icon, Fragment<'a>)>,
    danger: bool,
}

impl<'a> StatusBar<'a> {
    pub fn new(status: impl IntoFragment<'a>, status_icon: Icon) -> Self {
        Self {
            status: status.into_fragment(),
            status_icon,
            details: Vec::new(),
            danger: false,
        }
    }

    pub fn detail(mut self, icon: Icon, label: impl IntoFragment<'a>) -> Self {
        self.details.push((icon, label.into_fragment()));
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    fn header<Message: Clone + 'a>(self, toggle: Option<Message>) -> Element<'a, Message> {
        let danger = self.danger;
        let mut header = row![].spacing(spacing::LG).align_y(Vertical::Center);

        for (icon, label) in self.details {
            header = header.push(
                row![
                    status_icon(icon, false),
                    text(label).size(TEXT_SIZE).muted(),
                ]
                .spacing(spacing::SM)
                .align_y(Vertical::Center),
            );
        }

        header = header.extend([
            Space::new().width(Fill).into(),
            row![
                status_icon(self.status_icon, danger),
                text(self.status)
                    .size(TEXT_SIZE)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(if danger {
                            theme::colors(theme).danger
                        } else {
                            theme::colors(theme).muted
                        }),
                    }),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center)
            .into(),
        ]);

        if let Some(message) = toggle {
            header = header.push(
                Button::icon_only("Toggle log", Icon::Computer)
                    .diameter(ICON_SIZE)
                    .icon_size(ICON_SIZE)
                    .kind(ButtonKind::Transparent)
                    .on_press(message),
            );
        }

        column![
            rule::horizontal(1).style(|theme: &Theme| rule::Style {
                color: theme::colors(theme).border,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: true,
            }),
            container(header)
                .width(Fill)
                .padding([spacing::XS, spacing::MD]),
        ]
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<StatusBar<'a>> for Element<'a, Message> {
    fn from(status: StatusBar<'a>) -> Self {
        container(status.header(None))
            .width(Fill)
            .clip(true)
            .style(bar_style)
            .into()
    }
}

/// A Bottles status recipe with standard architecture and runner details.
pub struct LogPanel<'a> {
    status: StatusBar<'a>,
    log: Option<Fragment<'a>>,
}

impl<'a> LogPanel<'a> {
    pub fn new(
        architecture: impl IntoFragment<'a>,
        runner: impl IntoFragment<'a>,
        status: impl IntoFragment<'a>,
        status_icon: Icon,
    ) -> Self {
        Self {
            status: StatusBar::new(status, status_icon)
                .detail(Icon::Chip, architecture)
                .detail(Icon::Run, runner),
            log: None,
        }
    }

    pub fn log(mut self, log: impl IntoFragment<'a>) -> Self {
        self.log = Some(log.into_fragment());
        self
    }

    pub fn danger(mut self) -> Self {
        self.status = self.status.danger();
        self
    }
}

impl<'a, Message: 'static> From<LogPanel<'a>> for Element<'a, Message> {
    fn from(panel: LogPanel<'a>) -> Self {
        let header = panel.status.header(Some(Local::Toggle));
        let log: Element<'a, Local> = panel.log.map_or_else(
            || text("No activity yet").size(TEXT_SIZE).muted().into(),
            |log| text(log).size(TEXT_SIZE).into(),
        );
        let log = container(scrollable(log).height(Fill))
            .padding([spacing::MD, spacing::LG])
            .width(Fill)
            .style(|current_theme: &Theme| container::background(theme::hint(current_theme).color))
            .into();

        container(Element::new(LogWidget {
            children: [header, log],
        }))
        .width(Fill)
        .style(|current_theme: &Theme| {
            container::background(theme::colors(current_theme).background)
        })
        .into()
    }
}

fn status_icon<'a>(icon: Icon, danger: bool) -> svg::Svg<'a> {
    let icon = icon.view().width(ICON_SIZE).height(ICON_SIZE);

    if danger {
        icon.style(|theme: &Theme, _| svg::Style {
            color: Some(theme::colors(theme).danger),
        })
    } else {
        icon
    }
}

fn bar_style(theme: &Theme) -> container::Style {
    container::background(theme::colors(theme).background)
        .border(iced::Border::default().rounded(iced::border::bottom(6)))
}

#[derive(Debug, Clone, Copy)]
enum Local {
    Toggle,
}

struct LogWidget<'a> {
    children: [Element<'a, Local>; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pointer {
    Mouse,
    Touch(touch::Finger),
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    pointer: Pointer,
    origin_y: f32,
    origin_height: f32,
    restore_ratio: f32,
}

#[derive(Debug)]
struct State {
    open: bool,
    ratio: f32,
    available_height: f32,
    header_height: f32,
    drag: Option<Drag>,
    resize_focus: Interaction,
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            ratio: INITIAL_LOG_RATIO,
            available_height: 0.0,
            header_height: 0.0,
            drag: None,
            resize_focus: Interaction::default(),
        }
    }
}

impl State {
    fn toggle(&mut self) -> bool {
        if self.open {
            let restore_header = operation::Focusable::is_focused(&self.resize_focus);
            self.open = false;
            self.drag = None;
            operation::Focusable::unfocus(&mut self.resize_focus);
            restore_header
        } else {
            self.open = true;
            false
        }
    }

    fn expanded_height(&self) -> f32 {
        (self.available_height * self.ratio).clamp(self.header_height, self.available_height)
    }

    fn set_height(&mut self, height: f32) {
        if self.available_height > 0.0 {
            self.ratio = (height / self.available_height)
                .clamp(self.header_height / self.available_height, 1.0);
        }
    }

    fn resize_with_keyboard(&mut self, key: &keyboard::Key) -> Option<bool> {
        if !self.open
            || !self.has_visible_log()
            || !operation::Focusable::is_focused(&self.resize_focus)
        {
            return None;
        }

        let restore_header = match key.as_ref() {
            keyboard::Key::Named(key::Named::ArrowUp) => {
                self.set_height(self.expanded_height() + spacing::LG);
                false
            }
            keyboard::Key::Named(key::Named::ArrowDown) => {
                let height = self.expanded_height() - spacing::LG;

                if height <= self.header_height + f32::EPSILON {
                    self.toggle()
                } else {
                    self.set_height(height);
                    false
                }
            }
            keyboard::Key::Named(key::Named::Home) => self.toggle(),
            keyboard::Key::Named(key::Named::End) => {
                self.set_height(self.available_height);
                false
            }
            _ => return None,
        };

        Some(restore_header)
    }

    fn has_visible_log(&self) -> bool {
        self.available_height > self.header_height + f32::EPSILON
            && self.expanded_height() > self.header_height + f32::EPSILON
    }

    fn continue_resize(&mut self, event: &Event) -> Option<bool> {
        let drag = self.drag?;

        if matches!(event, Event::Window(window::Event::Unfocused))
            || matches!(
                (drag.pointer, event),
                (
                    Pointer::Touch(expected),
                    Event::Touch(touch::Event::FingerLost { id, .. })
                ) if expected == *id
            )
        {
            self.ratio = drag.restore_ratio;
            self.drag = None;
            return Some(false);
        }

        let position = match (drag.pointer, event) {
            (Pointer::Mouse, Event::Mouse(mouse::Event::CursorMoved { position })) => {
                Some(*position)
            }
            (
                Pointer::Touch(expected),
                Event::Touch(
                    touch::Event::FingerMoved { id, position }
                    | touch::Event::FingerLifted { id, position },
                ),
            ) if expected == *id => Some(*position),
            _ => None,
        };

        if let Some(position) = position {
            self.set_height(drag.origin_height + drag.origin_y - position.y);
        }

        let released = matches!(
            (drag.pointer, event),
            (
                Pointer::Mouse,
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            )
        ) || matches!(
            (drag.pointer, event),
            (
                Pointer::Touch(expected),
                Event::Touch(touch::Event::FingerLifted { id, .. })
            ) if expected == *id
        );

        if released {
            self.drag = None;

            if self.expanded_height() <= self.header_height + f32::EPSILON {
                self.ratio = drag.restore_ratio;
                return Some(self.toggle());
            }

            return Some(false);
        }

        matches!(event, Event::Mouse(_) | Event::Touch(_)).then_some(false)
    }

    fn start_resize(&mut self, event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
        let handle = resize_bounds(bounds);
        let pointer = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
                .position()
                .filter(|position| handle.contains(*position))
                .map(|position| (Pointer::Mouse, position)),
            Event::Touch(touch::Event::FingerPressed { id, position })
                if handle.contains(*position) =>
            {
                Some((Pointer::Touch(*id), *position))
            }
            _ => None,
        };

        if let Some((pointer, position)) = pointer {
            self.drag = Some(Drag {
                pointer,
                origin_y: position.y,
                origin_height: self.expanded_height(),
                restore_ratio: self.ratio,
            });
            true
        } else {
            false
        }
    }
}

impl<Message: 'static> Widget<Message, Theme, iced::Renderer> for LogWidget<'_> {
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
        let limits = limits.width(Fill).height(Length::Shrink);
        let header =
            self.children[0]
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits);
        let header_height = header.size().height;
        let max_height = limits.max().height;
        let available_height = if max_height.is_finite() {
            max_height.max(header_height)
        } else {
            (header_height + UNBOUNDED_LOG_HEIGHT) / INITIAL_LOG_RATIO
        };
        let state = tree.state.downcast_mut::<State>();
        state.available_height = available_height;
        state.header_height = header_height;

        if !state.has_visible_log() {
            operation::Focusable::unfocus(&mut state.resize_focus);
        }

        if !state.open {
            return layout::Node::with_children(header.size(), vec![header]);
        }

        let width = header.size().width;
        let log_height = state.expanded_height() - header_height;
        let log_size = Size::new(width, log_height);
        let log = self.children[1].as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(log_size, log_size),
        );

        layout::Node::with_children(
            Size::new(width, state.expanded_height()),
            vec![header, log.move_to(Point::new(0.0, header_height))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let (state, children) = (&mut tree.state, &mut tree.children);
        let state = state.downcast_mut::<State>();
        let open = state.open;
        let mut layouts = layout.children();
        let header_layout = layouts.next().expect("status header layout");
        let log_layout = open.then(|| layouts.next().expect("status log layout"));

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children[0].as_widget_mut().operate(
                &mut children[0],
                header_layout,
                renderer,
                operation,
            );

            if let Some(log_layout) = log_layout {
                if log_layout.bounds().height > 0.0 {
                    operation.focusable(
                        None,
                        resize_bounds(layout.bounds()),
                        &mut state.resize_focus,
                    );
                }
                self.children[1].as_widget_mut().operate(
                    &mut children[1],
                    log_layout,
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
        if matches!(event, Event::Window(window::Event::Unfocused)) {
            operation::Focusable::unfocus(&mut tree.state.downcast_mut::<State>().resize_focus);
        }

        let open = tree.state.downcast_ref::<State>().open;
        let mut layouts = layout.children();
        let header_layout = layouts.next().expect("status header layout");
        let resize = open.then(|| tree.state.downcast_mut::<State>().continue_resize(event));

        if let Some(restore_header) = resize.flatten() {
            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();

            if restore_header {
                focus_first_descendant(
                    self.children[0].as_widget_mut(),
                    &mut tree.children[0],
                    header_layout,
                    renderer,
                );
            }

            return;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && let Some(restore_header) =
                tree.state.downcast_mut::<State>().resize_with_keyboard(key)
        {
            if restore_header {
                focus_first_descendant(
                    self.children[0].as_widget_mut(),
                    &mut tree.children[0],
                    header_layout,
                    renderer,
                );
            }

            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();
            return;
        }

        let visible = 1 + usize::from(open);
        let mut messages = Vec::new();
        let mut local_shell = Shell::new(&mut messages);

        for ((child, child_tree), child_layout) in self.children[..visible]
            .iter_mut()
            .zip(&mut tree.children[..visible])
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                &mut local_shell,
                viewport,
            );
        }

        let captured = local_shell.is_event_captured();
        let layout_invalid = local_shell.is_layout_invalid();
        let widgets_invalid = local_shell.are_widgets_invalid();
        let redraw = local_shell.redraw_request();
        drop(local_shell);

        if captured {
            shell.capture_event();
        }
        if layout_invalid {
            shell.invalidate_layout();
        }
        if widgets_invalid {
            shell.invalidate_widgets();
        }
        shell.request_redraw_at(redraw);

        if !messages.is_empty() {
            let restore_header = tree.state.downcast_mut::<State>().toggle();

            if restore_header {
                focus_first_descendant(
                    self.children[0].as_widget_mut(),
                    &mut tree.children[0],
                    header_layout,
                    renderer,
                );
            }

            shell.invalidate_layout();
            shell.request_redraw();
        }

        if !captured
            && tree.state.downcast_ref::<State>().open
            && tree
                .state
                .downcast_mut::<State>()
                .start_resize(event, cursor, layout.bounds())
        {
            shell.capture_event();
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

        if state.open && (state.drag.is_some() || cursor.is_over(resize_bounds(layout.bounds()))) {
            return mouse::Interaction::ResizingVertically;
        }

        let visible = 1 + usize::from(state.open);

        self.children[..visible]
            .iter()
            .zip(&tree.children[..visible])
            .zip(layout.children())
            .map(|((child, child_tree), child_layout)| {
                child.as_widget().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
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
        let visible = 1 + usize::from(state.open);

        for ((child, child_tree), child_layout) in self.children[..visible]
            .iter()
            .zip(&tree.children[..visible])
            .zip(layout.children())
        {
            child.as_widget().draw(
                child_tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }

        if state.open && operation::Focusable::is_focused(&state.resize_focus) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        height: 2.0,
                        ..layout.bounds()
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                Background::Color(theme::colors(theme).accent_muted),
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
        let visible = 1 + usize::from(tree.state.downcast_ref::<State>().open);

        overlay::from_children(
            &mut self.children[..visible],
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
        .map(|overlay| overlay.map(&unexpected_message::<Message>))
    }
}

fn resize_bounds(bounds: Rectangle) -> Rectangle {
    Rectangle {
        height: RESIZE_TARGET_HEIGHT.min(bounds.height),
        ..bounds
    }
}

fn unexpected_message<Message>(_: Local) -> Message {
    unreachable!("log panel messages are handled locally")
}

#[cfg(test)]
mod tests {
    use super::{INITIAL_LOG_RATIO, State};

    #[test]
    fn disclosure_and_size_are_local() {
        let mut state = State {
            available_height: 600.0,
            header_height: 44.0,
            ..State::default()
        };

        assert!(!state.open);
        state.toggle();
        assert!(state.open);
        assert_eq!(state.expanded_height(), 600.0 * INITIAL_LOG_RATIO);

        state.set_height(300.0);
        state.toggle();
        state.toggle();
        assert_eq!(state.expanded_height(), 300.0);
    }
}
