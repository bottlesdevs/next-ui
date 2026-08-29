use iced::{
    Background, ContentFit, Element, Event, Fill, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    alignment::Vertical,
    touch,
    widget::{
        Space, container, row, scrollable,
        scrollable::{RelativeOffset, Viewport},
        svg, text,
        text::{Fragment, IntoFragment},
    },
    window,
};

use crate::{icons::Icon, theme};

use super::{
    button::{Button, ButtonKind},
    control::focus_first_descendant,
    spacing,
    text::TextExt as _,
};

const INITIAL_LOG_RATIO: f32 = 0.3;
const RESIZE_TARGET_HEIGHT: f32 = 8.0;
const BOTTOM_TOLERANCE: f32 = 1.0;
const UNBOUNDED_LOG_HEIGHT: f32 = 180.0;
const ICON_SIZE: f32 = 16.0;
const TEXT_SIZE: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl BottleStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Stopping => "Stopping",
            Self::Failed => "Failed",
        }
    }

    const fn icon(self) -> Icon {
        match self {
            Self::Stopped | Self::Stopping => Icon::Power,
            Self::Starting | Self::Running => Icon::Lightning,
            Self::Failed => Icon::Cross,
        }
    }
}

/// A bottle runtime footer showing environment status and collapsible logs.
pub struct StatusBar<'a> {
    architecture: Fragment<'a>,
    runner: Fragment<'a>,
    state: BottleStatus,
    log: Option<Fragment<'a>>,
}

impl<'a> StatusBar<'a> {
    pub fn new(
        architecture: impl IntoFragment<'a>,
        runner: impl IntoFragment<'a>,
        state: BottleStatus,
    ) -> Self {
        Self {
            architecture: architecture.into_fragment(),
            runner: runner.into_fragment(),
            state,
            log: None,
        }
    }

    pub fn log(mut self, log: impl IntoFragment<'a>) -> Self {
        self.log = Some(log.into_fragment());
        self
    }
}

impl<'a, Message: 'static> From<StatusBar<'a>> for Element<'a, Message> {
    fn from(status: StatusBar<'a>) -> Self {
        let header = row![
            row![
                status_icon(Icon::Chip),
                text(status.architecture).size(TEXT_SIZE).muted(),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
            row![
                status_icon(Icon::Run),
                text(status.runner).size(TEXT_SIZE).muted(),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
            Space::new().width(Fill),
            row![
                status_icon(status.state.icon()),
                text(status.state.label())
                    .size(TEXT_SIZE)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(if status.state == BottleStatus::Failed {
                            theme.palette().danger
                        } else {
                            theme.extended_palette().secondary.weak.text
                        }),
                    }),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
            Button::icon_only("Toggle log", Icon::Computer)
                .diameter(ICON_SIZE)
                .icon_size(ICON_SIZE)
                .kind(ButtonKind::Transparent)
                .on_press(Local::Toggle),
        ]
        .spacing(spacing::LG)
        .align_y(Vertical::Center);

        let header = container(header)
            .width(Fill)
            .padding([spacing::XS, spacing::MD])
            .into();
        let log: Element<'a, Local> = status.log.map_or_else(
            || text("No activity yet").size(TEXT_SIZE).muted().into(),
            |log| text(log).size(TEXT_SIZE).into(),
        );
        let log = container(scrollable(log).height(Fill).on_scroll(Local::Scrolled))
            .padding([spacing::MD, spacing::LG])
            .width(Fill)
            .style(|current_theme: &Theme| {
                container::background(theme::BottlesTheme::from(current_theme).hint.color)
                    .border(iced::Border::default().rounded(iced::border::bottom(6)))
            })
            .into();

        container(Element::new(StatusWidget {
            children: [header, log],
        }))
        .width(Fill)
        .clip(true)
        .style(|theme: &Theme| {
            container::background(theme.extended_palette().background.weaker.color)
                .border(iced::Border::default().rounded(iced::border::bottom(6)))
        })
        .into()
    }
}

fn status_icon<'a, Message: 'a>(icon: Icon) -> Element<'a, Message> {
    svg(icon.handle())
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .content_fit(ContentFit::Contain)
        .into()
}

#[derive(Debug, Clone, Copy)]
enum Local {
    Toggle,
    Scrolled(Viewport),
}

struct StatusWidget<'a> {
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
    expanded: bool,
    ratio: f32,
    available_height: f32,
    header_height: f32,
    drag: Option<Drag>,
    follow_tail: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expanded: false,
            ratio: INITIAL_LOG_RATIO,
            available_height: 0.0,
            header_height: 0.0,
            drag: None,
            follow_tail: true,
        }
    }
}

impl State {
    fn toggle(&mut self) {
        if self.expanded {
            self.collapse();
        } else {
            self.expanded = true;
        }
    }

    fn collapse(&mut self) {
        self.expanded = false;
        self.drag = None;
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
                self.collapse();
                return Some(true);
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

impl<Message: 'static> Widget<Message, Theme, iced::Renderer> for StatusWidget<'_> {
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

        if !state.expanded {
            return layout::Node::with_children(header.size(), vec![header]);
        }

        let height = state.expanded_height();
        let log_height = height - header_height;
        let log_size = Size::new(header.size().width, log_height);
        let log = self.children[1].as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(log_size, log_size),
        );

        layout::Node::with_children(
            Size::new(header.size().width, height),
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
        let expanded = tree.state.downcast_ref::<State>().expanded;
        let mut layouts = layout.children();
        let header_layout = layouts.next().expect("status header layout");

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children[0].as_widget_mut().operate(
                &mut tree.children[0],
                header_layout,
                renderer,
                operation,
            );

            if expanded {
                self.children[1].as_widget_mut().operate(
                    &mut tree.children[1],
                    layouts.next().expect("status log layout"),
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
        let expanded = tree.state.downcast_ref::<State>().expanded;
        let mut layouts = layout.children();
        let header_layout = layouts.next().expect("status header layout");
        let log_layout = expanded.then(|| layouts.next().expect("status log layout"));
        let resize = expanded.then(|| tree.state.downcast_mut::<State>().continue_resize(event));

        if let Some(collapsed) = resize.flatten() {
            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();

            if collapsed {
                focus_first_descendant(
                    self.children[0].as_widget_mut(),
                    &mut tree.children[0],
                    header_layout,
                    renderer,
                );
            }

            return;
        }

        let mut messages = Vec::new();
        let mut local_shell = Shell::new(&mut messages);
        let visible = 1 + usize::from(expanded);

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

        let mut snap_to_end = false;
        for message in messages {
            match message {
                Local::Toggle => {
                    tree.state.downcast_mut::<State>().toggle();
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                Local::Scrolled(viewport) => {
                    let at_bottom = is_at_bottom(viewport);
                    let state = tree.state.downcast_mut::<State>();

                    if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
                        snap_to_end |= state.follow_tail && !at_bottom;
                    } else {
                        state.follow_tail = at_bottom;
                    }
                }
            }
        }

        if snap_to_end && let Some(log_layout) = log_layout {
            self.children[1].as_widget_mut().operate(
                &mut tree.children[1],
                log_layout,
                renderer,
                &mut SnapToEnd,
            );
            shell.request_redraw();
        }

        if !captured
            && tree.state.downcast_ref::<State>().expanded
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

        if state.expanded
            && (state.drag.is_some() || cursor.is_over(resize_bounds(layout.bounds())))
        {
            return mouse::Interaction::ResizingVertically;
        }

        let visible = 1 + usize::from(state.expanded);
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
        let visible = 1 + usize::from(state.expanded);

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

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    height: 1.0,
                    ..layout.bounds()
                },
                snap: true,
                ..renderer::Quad::default()
            },
            Background::Color(theme.extended_palette().background.neutral.color),
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let expanded = tree.state.downcast_ref::<State>().expanded;
        let visible = 1 + usize::from(expanded);
        let children = &mut self.children[..visible];
        overlay::from_children(children, tree, layout, renderer, viewport, translation)
            .map(|overlay| overlay.map(&unexpected_status_message::<Message>))
    }
}

struct SnapToEnd;

impl Operation for SnapToEnd {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        state.snap_to(RelativeOffset::END.into());
    }
}

fn resize_bounds(bounds: Rectangle) -> Rectangle {
    Rectangle {
        height: RESIZE_TARGET_HEIGHT.min(bounds.height),
        ..bounds
    }
}

fn is_at_bottom(viewport: Viewport) -> bool {
    let offset = viewport.absolute_offset().y;
    let maximum = (viewport.content_bounds().height - viewport.bounds().height).max(0.0);

    maximum - offset <= BOTTOM_TOLERANCE
}

fn unexpected_status_message<Message>(_: Local) -> Message {
    unreachable!("status bar messages are handled locally")
}

#[cfg(test)]
mod tests {
    use super::{INITIAL_LOG_RATIO, State};

    #[test]
    fn disclosure_and_size_are_local() {
        let mut state = State::default();
        state.available_height = 600.0;
        state.header_height = 44.0;

        assert!(!state.expanded);
        state.toggle();
        assert!(state.expanded);
        assert_eq!(state.expanded_height(), 600.0 * INITIAL_LOG_RATIO);

        state.set_height(300.0);
        state.toggle();
        state.toggle();
        assert_eq!(state.expanded_height(), 300.0);
    }
}
