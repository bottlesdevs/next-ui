use std::f32::consts::PI;

use iced::{
    Element, Event, Point, Rectangle, Renderer, Theme,
    animation::{Animation, Easing},
    mouse,
    time::Instant,
    widget::{Action, canvas},
    window,
};

use crate::icons::SIZE;

const STROKE_WIDTH: f32 = 2.5;
const START_ANGLE: f32 = -PI / 2.0;

/// Circular percentage indicator, e.g. for an in-flight download or account link.
pub struct ProgressRing {
    progress: f32,
}

impl ProgressRing {
    /// `progress` is clamped to `0.0..=1.0`; `1.0` renders as a completed checkmark.
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
        }
    }
}

impl<'a, Message: 'a> From<ProgressRing> for Element<'a, Message> {
    fn from(ring: ProgressRing) -> Self {
        canvas::Canvas::new(AnimatedRing {
            progress: ring.progress,
        })
        .width(SIZE)
        .height(SIZE)
        .into()
    }
}

struct AnimatedRing {
    progress: f32,
}

#[derive(Debug, Default)]
struct RingState {
    animation: Option<Animation<f32>>,
}

impl RingState {
    fn sync(&mut self, progress: f32, now: Instant) -> bool {
        let animation = self
            .animation
            .get_or_insert_with(|| Animation::new(progress).quick().easing(Easing::EaseOut));

        if animation.value() != progress {
            animation.go_mut(progress, now);
        }

        animation.is_animating(now)
    }
}

impl<Message> canvas::Program<Message> for AnimatedRing {
    type State = RingState;

    fn update(
        &self,
        state: &mut RingState,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && state.sync(self.progress, *now)
        {
            Some(Action::request_redraw())
        } else {
            None
        }
    }

    fn draw(
        &self,
        state: &RingState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let progress = state
            .animation
            .as_ref()
            .map_or(self.progress, |animation| {
                animation.interpolate_with(|value| value, Instant::now())
            });
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - STROKE_WIDTH / 2.0;
        let palette = theme.extended_palette();
        let track = canvas::Path::circle(center, radius);

        frame.stroke(
            &track,
            canvas::Stroke::default()
                .with_width(STROKE_WIDTH)
                .with_color(palette.background.strong.color),
        );

        if progress >= 1.0 {
            let unit = radius * 0.5;
            let checkmark = canvas::Path::new(|builder| {
                builder.move_to(Point::new(center.x - unit * 0.9, center.y));
                builder.line_to(Point::new(center.x - unit * 0.2, center.y + unit * 0.7));
                builder.line_to(Point::new(center.x + unit, center.y - unit * 0.6));
            });

            frame.stroke(
                &checkmark,
                canvas::Stroke::default()
                    .with_width(STROKE_WIDTH)
                    .with_color(theme.palette().primary)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
        } else if progress > 0.0 {
            let arc = canvas::Path::new(|builder| {
                builder.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(START_ANGLE),
                    end_angle: iced::Radians(START_ANGLE + PI * 2.0 * progress),
                });
            });

            frame.stroke(
                &arc,
                canvas::Stroke::default()
                    .with_width(STROKE_WIDTH)
                    .with_color(theme.palette().primary)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }

        vec![frame.into_geometry()]
    }
}
