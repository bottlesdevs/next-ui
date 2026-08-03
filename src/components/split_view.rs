use iced::{
    Element, Event, Fill, Length, Padding, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    animation::{Animation, Easing},
    time::Instant,
    touch,
    widget::{container, responsive},
    window,
};

use super::spacing;

const BREAKPOINT: f32 = 1360.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneMode {
    Single,
    Split,
}

pub struct SplitView<'a, Message> {
    master: Box<dyn Fn(f32, PaneMode) -> Element<'a, Message> + 'a>,
    detail: Box<dyn Fn(f32, PaneMode) -> Element<'a, Message> + 'a>,
    show_detail: bool,
}

impl<'a, Message> SplitView<'a, Message> {
    pub fn new(
        master: impl Fn(f32, PaneMode) -> Element<'a, Message> + 'a,
        detail: impl Fn(f32, PaneMode) -> Element<'a, Message> + 'a,
    ) -> Self {
        Self {
            master: Box::new(master),
            detail: Box::new(detail),
            show_detail: false,
        }
    }

    pub fn show_detail(mut self, show_detail: bool) -> Self {
        self.show_detail = show_detail;
        self
    }
}

impl<'a, Message: 'a> From<SplitView<'a, Message>> for Element<'a, Message> {
    fn from(split_view: SplitView<'a, Message>) -> Self {
        let SplitView {
            master,
            detail,
            show_detail,
        } = split_view;

        responsive(move |size| {
            let wide = size.width >= BREAKPOINT;
            let width = content_width(size.width);
            let split_width = (width - spacing::SM).max(0.0);
            let master_mode = if wide && show_detail {
                PaneMode::Split
            } else {
                PaneMode::Single
            };
            let master_width = if wide && show_detail {
                split_width / 3.0
            } else {
                width
            };
            let detail_width = if wide { split_width * 2.0 / 3.0 } else { width };

            padded(
                AnimatedSplit::new(
                    master(master_width, master_mode),
                    detail(
                        detail_width,
                        if wide {
                            PaneMode::Split
                        } else {
                            PaneMode::Single
                        },
                    ),
                    show_detail,
                    wide,
                )
                .into(),
            )
        })
        .into()
    }
}

struct AnimatedSplit<'a, Message> {
    children: [Element<'a, Message>; 2],
    show_detail: bool,
    wide: bool,
}

impl<'a, Message> AnimatedSplit<'a, Message> {
    fn new(
        master: Element<'a, Message>,
        detail: Element<'a, Message>,
        show_detail: bool,
        wide: bool,
    ) -> Self {
        Self {
            children: [master, detail],
            show_detail,
            wide,
        }
    }
}

#[derive(Debug)]
struct State {
    transition: Animation<bool>,
}

impl State {
    fn new(show_detail: bool) -> Self {
        Self {
            transition: Animation::new(show_detail).quick().easing(Easing::EaseOut),
        }
    }

    fn progress(&self, now: Instant) -> f32 {
        self.transition.interpolate(0.0, 1.0, now)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for AnimatedSplit<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.show_detail))
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<State>();

        if state.transition.value() != self.show_detail {
            state.transition.go_mut(self.show_detail, Instant::now());
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let [master, detail] = pane_bounds(
            size,
            self.wide,
            tree.state.downcast_ref::<State>().progress(Instant::now()),
        );
        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip([master, detail])
            .map(|((child, tree), bounds)| {
                child
                    .as_widget_mut()
                    .layout(
                        tree,
                        renderer,
                        &layout::Limits::new(bounds.size(), bounds.size()),
                    )
                    .move_to(bounds.position())
            })
            .collect();

        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let bounds = layout.bounds();
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());
        operation.container(None, bounds);
        operation.traverse(&mut |operation| {
            for (_index, ((child, tree), child_layout)) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(index, (_, child_layout))| {
                    pane_is_active(*index, self.wide, progress)
                        && child_layout.bounds().intersects(&bounds)
                })
            {
                child
                    .as_widget_mut()
                    .operate(tree, child_layout, renderer, operation);
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
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && tree
                .state
                .downcast_ref::<State>()
                .transition
                .is_animating(*now)
        {
            shell.invalidate_layout();
            shell.request_redraw();
        }

        let bounds = layout.bounds();
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());

        for (index, ((child, tree), child_layout)) in self
            .children
            .iter_mut()
            .enumerate()
            .rev()
            .zip(tree.children.iter_mut().rev())
            .zip(layout.children().rev())
            .map(|(((index, child), tree), child_layout)| (index, ((child, tree), child_layout)))
            .filter(|(index, (_, child_layout))| {
                pane_is_active(*index, self.wide, progress)
                    && child_layout.bounds().intersects(&bounds)
            })
        {
            child.as_widget_mut().update(
                tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if shell.is_event_captured()
                || (index == 1 && pointer_is_over(event, cursor, child_layout.bounds()))
            {
                return;
            }
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
        let bounds = layout.bounds();
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());
        let mut layouts = layout.children();
        let master = layouts.next().expect("master layout");
        let detail = layouts.next().expect("detail layout");
        let index = if pane_is_active(1, self.wide, progress)
            && detail.bounds().intersects(&bounds)
            && cursor.is_over(detail.bounds())
        {
            1
        } else {
            0
        };
        let child_layout = [master, detail][index];

        self.children[index].as_widget().mouse_interaction(
            &tree.children[index],
            child_layout,
            cursor,
            viewport,
            renderer,
        )
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
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };

        renderer.with_layer(clip, |renderer| {
            for (index, ((child, tree), child_layout)) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(_, (_, child_layout))| child_layout.bounds().intersects(&clip))
            {
                let draw = |renderer: &mut iced::Renderer| {
                    child.as_widget().draw(
                        tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        &clip,
                    );
                };

                if index == 0 {
                    draw(renderer);
                } else {
                    renderer.with_layer(clip, draw);
                }
            }
        });
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());

        if progress <= 0.0 {
            return self.children[0].as_widget_mut().overlay(
                &mut tree.children[0],
                layout.children().next().expect("master layout"),
                renderer,
                viewport,
                translation,
            );
        }

        if !self.wide && progress >= 1.0 {
            return self.children[1].as_widget_mut().overlay(
                &mut tree.children[1],
                layout.children().nth(1).expect("detail layout"),
                renderer,
                viewport,
                translation,
            );
        }

        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<AnimatedSplit<'a, Message>> for Element<'a, Message> {
    fn from(split: AnimatedSplit<'a, Message>) -> Self {
        Element::new(split)
    }
}

fn pane_is_active(index: usize, wide: bool, progress: f32) -> bool {
    match index {
        0 => wide || progress < 1.0,
        1 => progress > 0.0,
        _ => false,
    }
}

fn pointer_is_over(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(_) => cursor.is_over(bounds),
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => bounds.contains(*position),
        _ => false,
    }
}

fn pane_bounds(size: Size, wide: bool, progress: f32) -> [Rectangle; 2] {
    if wide {
        let available = (size.width - spacing::SM).max(0.0);
        let split_master = available / 3.0;
        let detail_width = available * 2.0 / 3.0;
        let master_width = size.width + (split_master - size.width) * progress;

        [
            Rectangle::new(Point::ORIGIN, Size::new(master_width, size.height)),
            Rectangle::new(
                Point::new(master_width + spacing::SM, 0.0),
                Size::new(detail_width, size.height),
            ),
        ]
    } else {
        [
            Rectangle::new(Point::ORIGIN, size),
            Rectangle::new(
                Point::new(size.width * (1.0 - progress), 0.0),
                Size::new(size.width, size.height),
            ),
        ]
    }
}

fn content_width(width: f32) -> f32 {
    (width - 2.0 * spacing::SM).max(0.0)
}

fn padded<'a, Message: 'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .padding(page_padding())
        .into()
}

fn page_padding() -> Padding {
    Padding::ZERO.horizontal(spacing::SM).bottom(spacing::SM)
}
