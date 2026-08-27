use iced::{
    Background, Degrees, Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    animation::{Animation, Easing},
    gradient,
    time::Instant,
    widget::responsive,
    window,
};

use crate::{
    theme::WINDOW,
    widgets::{event_cursor, spacing},
};

const BREAKPOINT: f32 = 900.0;
const COMPACT_MAX_WIDTH: f32 = 420.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneMode {
    Single,
    Split,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    Master,
    Detail,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaneSide {
    Start,
    #[default]
    End,
}

pub struct SplitView<'a, Message> {
    master: Box<dyn Fn(PaneMode) -> Element<'a, Message> + 'a>,
    detail: Box<dyn Fn(PaneMode) -> Element<'a, Message> + 'a>,
    show_detail: bool,
    detail_side: PaneSide,
    compact: Pane,
    master_blocked: bool,
}

impl<'a, Message> SplitView<'a, Message> {
    pub fn new(
        master: impl Fn(PaneMode) -> Element<'a, Message> + 'a,
        detail: impl Fn(PaneMode) -> Element<'a, Message> + 'a,
    ) -> Self {
        Self {
            master: Box::new(master),
            detail: Box::new(detail),
            show_detail: false,
            detail_side: PaneSide::End,
            compact: Pane::Master,
            master_blocked: false,
        }
    }

    pub fn show_detail(mut self, show_detail: bool) -> Self {
        self.show_detail = show_detail;
        self
    }

    pub fn detail_side(mut self, side: PaneSide) -> Self {
        self.detail_side = side;
        self
    }

    pub fn compact(mut self, pane: Pane) -> Self {
        self.compact = pane;
        self
    }

    pub fn block_master(mut self) -> Self {
        self.master_blocked = true;
        self
    }
}

impl<'a, Message: 'a> From<SplitView<'a, Message>> for Element<'a, Message> {
    fn from(split_view: SplitView<'a, Message>) -> Self {
        let SplitView {
            master,
            detail,
            show_detail,
            detail_side,
            compact,
            master_blocked,
        } = split_view;

        responsive(move |size| {
            let wide = size.width >= BREAKPOINT;
            let master_mode = if wide && show_detail {
                PaneMode::Split
            } else {
                PaneMode::Single
            };

            Element::new(AnimatedSplit {
                children: [
                    master(master_mode),
                    detail(if wide {
                        PaneMode::Split
                    } else {
                        PaneMode::Single
                    }),
                ],
                show_detail,
                wide,
                detail_side,
                compact,
                master_blocked,
            })
        })
        .into()
    }
}

struct AnimatedSplit<'a, Message> {
    children: [Element<'a, Message>; 2],
    show_detail: bool,
    wide: bool,
    detail_side: PaneSide,
    compact: Pane,
    master_blocked: bool,
}

impl<'a, Message> AnimatedSplit<'a, Message> {
    fn master_is_blocked(&self, progress: f32) -> bool {
        self.master_blocked && (self.show_detail || progress > 0.0)
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

        if !self.wide {
            *state = State::new(self.show_detail);
        } else if state.transition.value() != self.show_detail {
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
            self.detail_side,
            self.compact,
            self.master_blocked,
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
        let master_blocked = self.master_is_blocked(progress);
        operation.container(None, bounds);
        operation.traverse(&mut |operation| {
            for (_index, ((child, tree), child_layout)) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(index, (_, child_layout))| {
                    pane_is_interactive(*index, self.wide, progress, master_blocked)
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
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_ref::<State>();

            if state.transition.is_animating(*now) {
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }

        let bounds = layout.bounds();
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());
        let master_blocked = self.master_is_blocked(progress);

        for (index, child_layout) in layout.children().enumerate().rev() {
            if !pane_is_interactive(index, self.wide, progress, master_blocked)
                || !child_layout.bounds().intersects(&bounds)
            {
                continue;
            }

            self.children[index].as_widget_mut().update(
                &mut tree.children[index],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if shell.is_event_captured()
                || (index == 1
                    && matches!(event, Event::Mouse(_) | Event::Touch(_))
                    && event_cursor(event, cursor).is_over(child_layout.bounds()))
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
        let master_blocked = self.master_is_blocked(progress);
        let index = if pane_is_interactive(1, self.wide, progress, master_blocked)
            && detail.bounds().intersects(&bounds)
            && cursor.is_over(detail.bounds())
        {
            1
        } else {
            0
        };

        if index == 0 && master_blocked {
            return mouse::Interaction::default();
        }

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
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());

        renderer.with_layer(clip, |renderer| {
            for (index, ((child, tree), child_layout)) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(index, (_, child_layout))| {
                    pane_is_active(*index, self.wide, progress)
                        && child_layout.bounds().intersects(&clip)
                })
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

                draw(renderer);

                if index == 0
                    && self.master_blocked
                    && progress > 0.0
                    && let Some(bounds) = child_layout.bounds().intersection(&clip)
                {
                    renderer.with_layer(bounds, |renderer| {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds,
                                ..renderer::Quad::default()
                            },
                            Background::from(
                                gradient::Linear::new(match self.detail_side {
                                    PaneSide::Start => Degrees(90.0),
                                    PaneSide::End => Degrees(270.0),
                                })
                                .add_stop(0.0, WINDOW.scale_alpha(0.2 * progress))
                                .add_stop(0.7, WINDOW.scale_alpha(0.45 * progress))
                                .add_stop(1.0, WINDOW.scale_alpha(progress)),
                            ),
                        );
                    });
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

        if self.master_is_blocked(progress) {
            if progress <= 0.0 {
                return None;
            }

            return self.children[1].as_widget_mut().overlay(
                &mut tree.children[1],
                layout.children().nth(1).expect("detail layout"),
                renderer,
                viewport,
                translation,
            );
        }

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

fn pane_is_active(index: usize, wide: bool, progress: f32) -> bool {
    match index {
        0 => wide || progress < 1.0,
        1 => progress > 0.0,
        _ => false,
    }
}

fn pane_is_interactive(index: usize, wide: bool, progress: f32, master_blocked: bool) -> bool {
    pane_is_active(index, wide, progress) && (index != 0 || !master_blocked)
}

fn pane_bounds(
    size: Size,
    wide: bool,
    progress: f32,
    detail_side: PaneSide,
    compact: Pane,
    master_blocked: bool,
) -> [Rectangle; 2] {
    if wide {
        let available = (size.width - spacing::SM).max(0.0);
        let compact_width = (available / 3.0).min(COMPACT_MAX_WIDTH);
        let fill_width = available - compact_width;

        if compact == Pane::Detail && master_blocked {
            let offset = (compact_width + spacing::SM) * progress;
            let (master_x, detail_x) = match detail_side {
                PaneSide::Start => (offset, offset - compact_width - spacing::SM),
                PaneSide::End => (-offset, size.width + spacing::SM - offset),
            };

            return [
                Rectangle::new(Point::new(master_x, 0.0), size),
                Rectangle::new(
                    Point::new(detail_x, 0.0),
                    Size::new(compact_width, size.height),
                ),
            ];
        }

        let master_width = compact_width;
        let detail_width = fill_width;

        match detail_side {
            PaneSide::End => {
                let current_master_width = size.width + (master_width - size.width) * progress;

                [
                    Rectangle::new(Point::ORIGIN, Size::new(current_master_width, size.height)),
                    Rectangle::new(
                        Point::new(current_master_width + spacing::SM, 0.0),
                        Size::new(detail_width, size.height),
                    ),
                ]
            }
            PaneSide::Start => {
                let offset = (detail_width + spacing::SM) * progress;

                [
                    Rectangle::new(
                        Point::new(offset, 0.0),
                        Size::new(
                            size.width + (master_width - size.width) * progress,
                            size.height,
                        ),
                    ),
                    Rectangle::new(
                        Point::new(-(detail_width + spacing::SM) * (1.0 - progress), 0.0),
                        Size::new(detail_width, size.height),
                    ),
                ]
            }
        }
    } else {
        [
            Rectangle::new(Point::ORIGIN, size),
            Rectangle::new(
                Point::new(
                    match detail_side {
                        PaneSide::Start => -size.width,
                        PaneSide::End => size.width,
                    } * (1.0 - progress),
                    0.0,
                ),
                Size::new(size.width, size.height),
            ),
        ]
    }
}
