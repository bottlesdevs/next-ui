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
    theme,
    ui::chrome::WINDOW_CONTROL_AT_START,
    widgets::{event_cursor, header_bar::HeaderBar, spacing},
};

const BREAKPOINT: f32 = 900.0;
const COMPACT_MAX_WIDTH: f32 = 420.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaneContext {
    standalone: bool,
    owns_window_control: bool,
}

impl PaneContext {
    pub(super) fn is_standalone(self) -> bool {
        self.standalone
    }

    pub(super) fn header<'a, Message>(self, on_drag: Message) -> HeaderBar<'a, Message> {
        if self.owns_window_control {
            HeaderBar::new(on_drag)
        } else {
            HeaderBar::without_window_control(on_drag)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Navigation(PaneContext),
    SidePanel(Side),
}

impl Kind {
    fn base_is_blocked(self, show_second: bool, progress: f32) -> bool {
        matches!(self, Self::SidePanel(_)) && (show_second || progress > 0.0)
    }

    fn contexts(
        self,
        wide: bool,
        show_second: bool,
        window_control_at_start: bool,
    ) -> [PaneContext; 2] {
        let first_standalone = !(wide && show_second);
        let second_standalone = !wide;

        let [first_owns_control, second_owns_control] = match self {
            Self::Navigation(parent) if !show_second => [parent.owns_window_control, false],
            Self::Navigation(parent) if !wide => [false, parent.owns_window_control],
            Self::Navigation(parent) if window_control_at_start => {
                [parent.owns_window_control, false]
            }
            Self::Navigation(parent) => [false, parent.owns_window_control],
            Self::SidePanel(_) if !show_second => [true, false],
            Self::SidePanel(_) if !wide => [false, true],
            Self::SidePanel(side) => [false, side.owns(window_control_at_start)],
        };

        [
            PaneContext {
                standalone: first_standalone,
                owns_window_control: first_owns_control,
            },
            PaneContext {
                standalone: second_standalone,
                owns_window_control: second_owns_control,
            },
        ]
    }
}

impl Side {
    fn owns(self, window_control_at_start: bool) -> bool {
        matches!(self, Self::Start) == window_control_at_start
    }
}

pub(super) fn navigation_split<'a, Message: 'a>(
    parent: PaneContext,
    show_detail: bool,
    master: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
    detail: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    adaptive_split(Kind::Navigation(parent), show_detail, master, detail)
}

pub(super) fn side_panel<'a, Message: 'a>(
    side: Side,
    open: bool,
    base: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
    panel: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    adaptive_split(Kind::SidePanel(side), open, base, panel)
}

fn adaptive_split<'a, Message: 'a>(
    kind: Kind,
    show_second: bool,
    first: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
    second: impl Fn(PaneContext) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    responsive(move |size| {
        let wide = size.width >= BREAKPOINT;
        let [first_context, second_context] =
            kind.contexts(wide, show_second, WINDOW_CONTROL_AT_START);

        Element::new(AnimatedSplit {
            children: [first(first_context), second(second_context)],
            show_second,
            wide,
            kind,
        })
    })
    .into()
}

struct AnimatedSplit<'a, Message> {
    children: [Element<'a, Message>; 2],
    show_second: bool,
    wide: bool,
    kind: Kind,
}

impl<'a, Message> AnimatedSplit<'a, Message> {
    fn base_is_blocked(&self, progress: f32) -> bool {
        self.kind.base_is_blocked(self.show_second, progress)
    }
}

#[derive(Debug)]
struct State {
    transition: Animation<bool>,
}

impl State {
    fn new(show_second: bool) -> Self {
        Self {
            transition: Animation::new(show_second).quick().easing(Easing::EaseOut),
        }
    }

    fn progress(&self, now: Instant) -> f32 {
        self.transition.interpolate(0.0, 1.0, now)
    }

    fn sync(&mut self, show_second: bool, wide: bool, kind: Kind, now: Instant) {
        if !wide && matches!(kind, Kind::SidePanel(_)) {
            *self = Self::new(show_second);
        } else if self.transition.value() != show_second {
            self.transition.go_mut(show_second, now);
        }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for AnimatedSplit<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.show_second))
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        tree.state.downcast_mut::<State>().sync(
            self.show_second,
            self.wide,
            self.kind,
            Instant::now(),
        );
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
        let [first, second] = pane_bounds(
            size,
            self.wide,
            tree.state.downcast_ref::<State>().progress(Instant::now()),
            self.kind,
        );
        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip([first, second])
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
        let base_blocked = self.base_is_blocked(progress);
        operation.container(None, bounds);
        operation.traverse(&mut |operation| {
            for (_index, ((child, tree), child_layout)) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(index, (_, child_layout))| {
                    pane_is_interactive(*index, self.wide, progress, base_blocked)
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
        let base_blocked = self.base_is_blocked(progress);

        for (index, child_layout) in layout.children().enumerate().rev() {
            if !pane_is_interactive(index, self.wide, progress, base_blocked)
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
        let base_blocked = self.base_is_blocked(progress);
        let index = if pane_is_interactive(1, self.wide, progress, base_blocked)
            && detail.bounds().intersects(&bounds)
            && cursor.is_over(detail.bounds())
        {
            1
        } else {
            0
        };

        if index == 0 && base_blocked {
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
                    && let Kind::SidePanel(side) = self.kind
                    && progress > 0.0
                    && let Some(bounds) = child_layout.bounds().intersection(&clip)
                {
                    let window = theme::window_color(theme);
                    renderer.with_layer(bounds, |renderer| {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds,
                                ..renderer::Quad::default()
                            },
                            Background::from(
                                gradient::Linear::new(match side {
                                    Side::Start => Degrees(90.0),
                                    Side::End => Degrees(270.0),
                                })
                                .add_stop(0.0, window.scale_alpha(0.2 * progress))
                                .add_stop(0.7, window.scale_alpha(0.45 * progress))
                                .add_stop(1.0, window.scale_alpha(progress)),
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

        if self.base_is_blocked(progress) {
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

fn pane_is_interactive(index: usize, wide: bool, progress: f32, base_blocked: bool) -> bool {
    pane_is_active(index, wide, progress) && (index != 0 || !base_blocked)
}

fn pane_bounds(size: Size, wide: bool, progress: f32, kind: Kind) -> [Rectangle; 2] {
    if !wide {
        let side = match kind {
            Kind::Navigation(_) => Side::End,
            Kind::SidePanel(side) => side,
        };

        return [
            Rectangle::new(Point::ORIGIN, size),
            Rectangle::new(
                Point::new(
                    match side {
                        Side::Start => -size.width,
                        Side::End => size.width,
                    } * (1.0 - progress),
                    0.0,
                ),
                size,
            ),
        ];
    }

    let available = (size.width - spacing::SM).max(0.0);
    let compact_width = (available / 3.0).min(COMPACT_MAX_WIDTH);

    match kind {
        Kind::Navigation(_) => {
            let detail_width = available - compact_width;
            let master_width = size.width + (compact_width - size.width) * progress;

            [
                Rectangle::new(Point::ORIGIN, Size::new(master_width, size.height)),
                Rectangle::new(
                    Point::new(master_width + spacing::SM, 0.0),
                    Size::new(detail_width, size.height),
                ),
            ]
        }
        Kind::SidePanel(side) => {
            let offset = (compact_width + spacing::SM) * progress;
            let (base_x, panel_x) = match side {
                Side::Start => (offset, offset - compact_width - spacing::SM),
                Side::End => (-offset, size.width + spacing::SM - offset),
            };

            [
                Rectangle::new(Point::new(base_x, 0.0), size),
                Rectangle::new(
                    Point::new(panel_x, 0.0),
                    Size::new(compact_width, size.height),
                ),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_context() -> PaneContext {
        PaneContext {
            standalone: true,
            owns_window_control: true,
        }
    }

    fn owners(contexts: [PaneContext; 2]) -> [bool; 2] {
        contexts.map(|context| context.owns_window_control)
    }

    #[test]
    fn navigation_assigns_the_window_control_to_the_visible_platform_edge() {
        let navigation = Kind::Navigation(root_context());

        assert_eq!(
            owners(navigation.contexts(true, false, true)),
            [true, false]
        );
        assert_eq!(owners(navigation.contexts(true, true, true)), [true, false]);
        assert_eq!(
            owners(navigation.contexts(true, true, false)),
            [false, true]
        );
        assert_eq!(
            owners(navigation.contexts(false, true, true)),
            [false, true]
        );

        let nested = Kind::Navigation(PaneContext {
            standalone: false,
            owns_window_control: false,
        });
        assert_eq!(owners(nested.contexts(true, true, false)), [false, false]);
    }

    #[test]
    fn side_panels_reserve_only_the_window_control_edge_they_cover() {
        let start = Kind::SidePanel(Side::Start);
        let end = Kind::SidePanel(Side::End);

        assert_eq!(owners(start.contexts(true, false, true)), [true, false]);
        assert_eq!(owners(start.contexts(false, true, false)), [false, true]);
        assert_eq!(owners(start.contexts(true, true, true)), [false, true]);
        assert_eq!(owners(start.contexts(true, true, false)), [false, false]);
        assert_eq!(owners(end.contexts(true, true, true)), [false, false]);
        assert_eq!(owners(end.contexts(true, true, false)), [false, true]);
    }

    #[test]
    fn wide_navigation_uses_a_one_third_master_capped_at_420() {
        let medium = Size::new(1200.0, 1000.0);
        let size = Size::new(1600.0, 1000.0);

        let navigation = Kind::Navigation(root_context());
        let [medium_master, medium_detail] = pane_bounds(medium, true, 1.0, navigation);
        let [closed_master, closed_detail] = pane_bounds(size, true, 0.0, navigation);
        let [open_master, open_detail] = pane_bounds(size, true, 1.0, navigation);

        assert_eq!(medium_master.size(), Size::new(396.0, 1000.0));
        assert_eq!(medium_detail.position(), Point::new(408.0, 0.0));
        assert_eq!(medium_detail.size(), Size::new(792.0, 1000.0));
        assert_eq!(closed_master, Rectangle::new(Point::ORIGIN, size));
        assert_eq!(closed_detail.position(), Point::new(1612.0, 0.0));
        assert_eq!(open_master.size(), Size::new(420.0, 1000.0));
        assert_eq!(open_detail.position(), Point::new(432.0, 0.0));
        assert_eq!(open_detail.size(), Size::new(1168.0, 1000.0));
        assert!(pane_is_interactive(0, true, 1.0, false));
        assert!(pane_is_interactive(1, true, 1.0, false));
    }

    #[test]
    fn narrow_navigation_endpoints_use_full_window_pages() {
        let size = Size::new(720.0, 600.0);

        let navigation = Kind::Navigation(root_context());
        let [closed_master, closed_detail] = pane_bounds(size, false, 0.0, navigation);
        let [open_master, open_detail] = pane_bounds(size, false, 1.0, navigation);

        assert_eq!(closed_master, Rectangle::new(Point::ORIGIN, size));
        assert_eq!(closed_detail, Rectangle::new(Point::new(720.0, 0.0), size));
        assert_eq!(open_master, Rectangle::new(Point::ORIGIN, size));
        assert_eq!(open_detail, Rectangle::new(Point::ORIGIN, size));
    }

    #[test]
    fn side_panels_enter_from_either_edge_without_resizing_the_base() {
        let size = Size::new(1600.0, 1000.0);

        let [start_closed_base, start_closed_panel] =
            pane_bounds(size, true, 0.0, Kind::SidePanel(Side::Start));
        let [start_open_base, start_open_panel] =
            pane_bounds(size, true, 1.0, Kind::SidePanel(Side::Start));
        let [end_closed_base, end_closed_panel] =
            pane_bounds(size, true, 0.0, Kind::SidePanel(Side::End));
        let [end_open_base, end_open_panel] =
            pane_bounds(size, true, 1.0, Kind::SidePanel(Side::End));

        assert_eq!(start_closed_base, Rectangle::new(Point::ORIGIN, size));
        assert_eq!(start_closed_panel.position(), Point::new(-432.0, 0.0));
        assert_eq!(start_open_base.position(), Point::new(432.0, 0.0));
        assert_eq!(start_open_base.size(), size);
        assert_eq!(start_open_panel.position(), Point::ORIGIN);
        assert_eq!(start_open_panel.size(), Size::new(420.0, 1000.0));

        assert_eq!(end_closed_base, Rectangle::new(Point::ORIGIN, size));
        assert_eq!(end_closed_panel.position(), Point::new(1612.0, 0.0));
        assert_eq!(end_open_base.position(), Point::new(-432.0, 0.0));
        assert_eq!(end_open_base.size(), size);
        assert_eq!(end_open_panel.position(), Point::new(1180.0, 0.0));
        assert_eq!(end_open_panel.size(), Size::new(420.0, 1000.0));
    }

    #[test]
    fn side_panels_block_the_base_until_the_transition_finishes() {
        let side_panel = Kind::SidePanel(Side::End);

        assert!(!side_panel.base_is_blocked(false, 0.0));
        assert!(side_panel.base_is_blocked(true, 0.0));
        assert!(side_panel.base_is_blocked(false, 0.5));
        assert!(!Kind::Navigation(root_context()).base_is_blocked(true, 1.0));
        assert!(!pane_is_interactive(0, true, 1.0, true));
        assert!(pane_is_interactive(1, true, 1.0, true));
    }

    #[test]
    fn narrow_side_panel_changes_are_immediate() {
        let now = Instant::now();
        let mut state = State::new(false);

        state.sync(true, false, Kind::SidePanel(Side::Start), now);
        assert!(state.transition.value());
        assert!(!state.transition.is_animating(now));
        assert_eq!(state.progress(now), 1.0);

        state.sync(false, false, Kind::SidePanel(Side::Start), now);
        assert!(!state.transition.value());
        assert!(!state.transition.is_animating(now));
        assert_eq!(state.progress(now), 0.0);
    }

    #[test]
    fn narrow_navigation_changes_remain_animated() {
        let now = Instant::now();
        let mut state = State::new(false);

        state.sync(true, false, Kind::Navigation(root_context()), now);

        assert!(state.transition.value());
        assert!(state.transition.is_animating(now));
        assert_eq!(state.progress(now), 0.0);
    }
}
