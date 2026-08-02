use iced::{
    Background, Border, Element, Event, Fill, Point, Rectangle, Renderer, Theme,
    animation::{Animation, Easing},
    mouse,
    theme::palette,
    time::Instant,
    widget::{Action, button, canvas},
    window,
};

use super::pressable::{Pressable, Status};

const WIDTH: f32 = 52.0;
const HEIGHT: f32 = 32.0;
const KNOB: f32 = 24.0;

pub struct Switcher<'a, Message> {
    is_on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> Switcher<'a, Message> {
    pub fn new(is_on: bool) -> Self {
        Self {
            is_on,
            on_toggle: None,
        }
    }

    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn on_toggle_maybe(mut self, on_toggle: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.on_toggle = on_toggle.map(|on_toggle| Box::new(on_toggle) as _);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Switcher<'a, Message>> for Element<'a, Message> {
    fn from(switcher: Switcher<'a, Message>) -> Self {
        let active = switcher.on_toggle.is_some();
        let knob = canvas::Canvas::new(AnimatedKnob {
            is_on: switcher.is_on,
            enabled: active,
        })
        .width(Fill)
        .height(Fill);
        let message = switcher
            .on_toggle
            .map(|on_toggle| on_toggle(!switcher.is_on));

        Pressable::new(knob)
            .width(WIDTH)
            .height(HEIGHT)
            .padding((HEIGHT - KNOB) / 2.0)
            .on_press_maybe(message)
            .style(track_style)
            .into()
    }
}

struct AnimatedKnob {
    is_on: bool,
    enabled: bool,
}

#[derive(Debug, Default)]
struct KnobState {
    animation: Option<Animation<bool>>,
}

impl KnobState {
    fn sync(&mut self, is_on: bool, now: Instant) -> bool {
        let animation = self
            .animation
            .get_or_insert_with(|| Animation::new(is_on).very_quick().easing(Easing::EaseOut));

        if animation.value() != is_on {
            animation.go_mut(is_on, now);
        }

        animation.is_animating(now)
    }
}

impl<Message> canvas::Program<Message> for AnimatedKnob {
    type State = KnobState;

    fn update(
        &self,
        state: &mut KnobState,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && state.sync(self.is_on, *now)
        {
            Some(Action::request_redraw())
        } else {
            None
        }
    }

    fn draw(
        &self,
        state: &KnobState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let progress = state.animation.as_ref().map_or_else(
            || if self.is_on { 1.0 } else { 0.0 },
            |animation| animation.interpolate(0.0, 1.0, Instant::now()),
        );
        let diameter = bounds.height;
        let color = if self.enabled {
            palette::mix(
                theme.extended_palette().background.stronger.color,
                theme.palette().primary,
                progress,
            )
        } else {
            theme.extended_palette().secondary.weak.text
        };
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        frame.fill(
            &canvas::Path::circle(
                Point::new(
                    diameter / 2.0 + (bounds.width - diameter) * progress,
                    diameter / 2.0,
                ),
                diameter / 2.0,
            ),
            color,
        );

        vec![frame.into_geometry()]
    }
}

fn track_style(theme: &Theme, _status: Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.weaker.color,
        )),
        border: Border::default().rounded(HEIGHT / 2.0),
        ..button::Style::default()
    }
}
