use iced::{
    Alignment, Element, Event, Fill, Length, Padding, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    alignment::Vertical,
    widget::{Space, container, row, scrollable, text},
};

use crate::{icons::Icon, theme};

use super::{button::Button, spacing, text::TextExt as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl StatusState {
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

pub struct StatusBar<'a> {
    architecture: &'a str,
    runner: &'a str,
    state: StatusState,
    log: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    pub fn new(architecture: &'a str, runner: &'a str, state: StatusState) -> Self {
        Self {
            architecture,
            runner,
            state,
            log: None,
        }
    }

    pub fn log(mut self, log: &'a str) -> Self {
        self.log = Some(log);
        self
    }
}

impl<'a, Message: 'static> From<StatusBar<'a>> for Element<'a, Message> {
    fn from(status: StatusBar<'a>) -> Self {
        let mut header = row![
            row![
                Icon::Chip.view(),
                text(status.architecture).supporting().muted(),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
            row![Icon::Run.view(), text(status.runner).supporting().muted(),]
                .spacing(spacing::SM)
                .align_y(Vertical::Center),
            Space::new().width(Fill),
            row![
                status.state.icon().view(),
                text(status.state.label())
                    .supporting()
                    .style(move |theme: &Theme| text::Style {
                        color: Some(if status.state == StatusState::Failed {
                            theme.palette().danger
                        } else {
                            theme.extended_palette().secondary.weak.text
                        }),
                    }),
            ]
            .spacing(spacing::SM)
            .align_y(Vertical::Center),
        ]
        .spacing(spacing::LG)
        .align_y(Vertical::Center);

        if status.log.is_some() {
            header = header.push(
                Button::icon_only("Toggle log", Icon::Computer)
                    .diameter(32.0)
                    .on_press(()),
            );
        }

        let header: Element<'a, ()> = container(header).padding([spacing::XS, spacing::LG]).into();
        let mut children = vec![header];

        if let Some(log) = status.log {
            children.push(
                container(scrollable(text(log).supporting()).height(Fill))
                    .padding([spacing::MD, spacing::LG])
                    .width(Fill)
                    .max_height(180)
                    .style(|current_theme: &Theme| {
                        container::background(theme::BottlesTheme::from(current_theme).hint.color)
                    })
                    .into(),
            );
        }

        container(Element::new(StatusWidget {
            children,
            status: status.state,
        }))
        .width(Fill)
        .clip(true)
        .style(theme::panel)
        .into()
    }
}

struct StatusWidget<'a> {
    children: Vec<Element<'a, ()>>,
    status: StatusState,
}

impl StatusWidget<'_> {
    fn has_log(&self) -> bool {
        self.children.len() > 1
    }
}

#[derive(Debug)]
struct State {
    expanded: bool,
    had_log: bool,
    status: StatusState,
}

impl State {
    fn new(has_log: bool, status: StatusState) -> Self {
        Self {
            expanded: has_log,
            had_log: has_log,
            status,
        }
    }

    fn reconcile(&mut self, has_log: bool, status: StatusState) {
        if has_log
            && (!self.had_log
                || (self.status != StatusState::Failed && status == StatusState::Failed))
        {
            self.expanded = true;
        } else if !has_log {
            self.expanded = false;
        }

        self.had_log = has_log;
        self.status = status;
    }
}

impl<Message: 'static> Widget<Message, Theme, iced::Renderer> for StatusWidget<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.has_log(), self.status))
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        tree.state
            .downcast_mut::<State>()
            .reconcile(self.has_log(), self.status);
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
        let state = tree.state.downcast_ref::<State>();
        let visible = 1 + usize::from(state.expanded && self.has_log());

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            limits,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Alignment::Start,
            &mut self.children[..visible],
            &mut tree.children[..visible],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
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
        viewport: &iced::Rectangle,
    ) {
        let mut messages = Vec::new();
        let mut local_shell = Shell::new(&mut messages);

        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
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

        if local_shell.is_empty() {
            shell.merge(local_shell, unexpected_status_message);
        } else {
            let state = tree.state.downcast_mut::<State>();
            state.expanded = !state.expanded;
            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
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
        viewport: &iced::Rectangle,
    ) {
        for ((child, child_tree), child_layout) in self
            .children
            .iter()
            .zip(&tree.children)
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
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &iced::Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let overlays = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((child, child_tree), child_layout)| {
                child
                    .as_widget_mut()
                    .overlay(child_tree, child_layout, renderer, viewport, translation)
                    .map(|overlay| overlay.map(&unexpected_status_message::<Message>))
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

fn unexpected_status_message<Message>((): ()) -> Message {
    unreachable!("status bar messages are handled locally")
}

#[cfg(test)]
mod tests {
    use super::{State, StatusState};

    #[test]
    fn disclosure_opens_for_new_logs_and_failures_without_overriding_user_choice() {
        let mut state = State::new(false, StatusState::Starting);

        state.reconcile(true, StatusState::Starting);
        assert!(state.expanded);

        state.expanded = false;
        state.reconcile(true, StatusState::Starting);
        assert!(!state.expanded);

        state.reconcile(true, StatusState::Failed);
        assert!(state.expanded);

        state.expanded = false;
        state.reconcile(true, StatusState::Failed);
        assert!(!state.expanded);
    }
}
