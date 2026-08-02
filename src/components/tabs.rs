use iced::{
    Alignment, Background, Border, Element, Event, Fill, Point, Rectangle, Renderer, Size, Theme,
    animation::{Animation, Easing},
    mouse,
    time::Instant,
    widget::{Action, Space, button, canvas, column, row, stack, text},
    window,
};

use super::{
    pressable::{Pressable, Status},
    spacing,
    text::TextExt as _,
};

pub struct Tab<'a, T> {
    value: T,
    label: &'a str,
    enabled: bool,
}

impl<'a, T> Tab<'a, T> {
    pub fn new(value: T, label: &'a str) -> Self {
        Self {
            value,
            label,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub struct Tabs<'a, T, Message> {
    tabs: Vec<Tab<'a, T>>,
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
}

impl<'a, T, Message> Tabs<'a, T, Message> {
    pub fn new(
        tabs: impl IntoIterator<Item = Tab<'a, T>>,
        selected: Option<T>,
        on_select: impl Fn(T) -> Message + 'a,
    ) -> Self {
        Self {
            tabs: tabs.into_iter().collect(),
            selected,
            on_select: Box::new(on_select),
        }
    }
}

impl<'a, T, Message> From<Tabs<'a, T, Message>> for Element<'a, Message>
where
    T: PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(tabs: Tabs<'a, T, Message>) -> Self {
        let Tabs {
            tabs,
            selected,
            on_select,
        } = tabs;
        let tab_count = tabs.len();
        let selected_index = selected
            .as_ref()
            .and_then(|selected| tabs.iter().position(|tab| &tab.value == selected));
        let children = tabs.into_iter().map(|tab| {
            let selected = selected.as_ref() == Some(&tab.value);
            let message = tab.enabled.then(|| on_select(tab.value));

            Pressable::new(
                column![
                    text(tab.label).label(),
                    Space::new().width(Fill).height(INDICATOR_HEIGHT),
                ]
                .align_x(Alignment::Center)
                .spacing(spacing::XS),
            )
            .width(Fill)
            .padding([spacing::XS, spacing::MD])
            .on_press_maybe(message)
            .style(move |theme, status| tab_style(theme, status, selected))
            .into()
        });

        stack![
            row(children).width(Fill),
            canvas::Canvas::new(TabIndicator {
                selected_index,
                tab_count,
            })
            .width(Fill)
            .height(Fill),
        ]
        .width(Fill)
        .into()
    }
}

const INDICATOR_HEIGHT: f32 = 3.0;

struct TabIndicator {
    selected_index: Option<usize>,
    tab_count: usize,
}

#[derive(Debug, Default)]
struct IndicatorState {
    animation: Option<Animation<f32>>,
}

impl IndicatorState {
    fn sync(&mut self, selected_index: Option<usize>, now: Instant) -> bool {
        let Some(selected_index) = selected_index else {
            self.animation = None;
            return false;
        };
        let selected_index = selected_index as f32;
        let animation = self.animation.get_or_insert_with(|| {
            Animation::new(selected_index)
                .very_quick()
                .easing(Easing::EaseOut)
        });

        if animation.value() != selected_index {
            animation.go_mut(selected_index, now);
        }

        animation.is_animating(now)
    }
}

impl<Message> canvas::Program<Message> for TabIndicator {
    type State = IndicatorState;

    fn update(
        &self,
        state: &mut IndicatorState,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && state.sync(self.selected_index, *now)
        {
            Some(Action::request_redraw())
        } else {
            None
        }
    }

    fn draw(
        &self,
        state: &IndicatorState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let Some(selected_index) = self.selected_index else {
            return Vec::new();
        };
        if self.tab_count == 0 {
            return Vec::new();
        }

        let position = state
            .animation
            .as_ref()
            .map_or(selected_index as f32, |animation| {
                animation.interpolate_with(|index| index, Instant::now())
            });
        let tab_width = bounds.width / self.tab_count as f32;
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        frame.fill_rectangle(
            Point::new(
                tab_width * position + spacing::MD,
                bounds.height - spacing::XS - INDICATOR_HEIGHT,
            ),
            Size::new(tab_width - 2.0 * spacing::MD, INDICATOR_HEIGHT),
            theme.palette().primary,
        );

        vec![frame.into_geometry()]
    }
}

fn tab_style(theme: &Theme, status: Status, selected: bool) -> button::Style {
    button::Style {
        background: matches!(status, Status::Focused).then_some(Background::Color(
            theme.extended_palette().background.strong.color,
        )),
        text_color: if selected
            || matches!(status, Status::Hovered | Status::Pressed | Status::Focused)
        {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(4),
        ..button::Style::default()
    }
}
