use iced::{
    Alignment, Background, Border, Element, Event, Fill, Padding, Point, Rectangle, Renderer, Size,
    Theme,
    advanced::text::{self as advanced_text, Paragraph as _, Renderer as _},
    alignment,
    animation::{Animation, Easing},
    mouse,
    time::Instant,
    widget::{Action, Space, button, canvas, column, container, row, stack, text},
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
        let labels = tabs.iter().map(|tab| tab.label).collect();
        let selected_index = selected
            .as_ref()
            .and_then(|selected| tabs.iter().position(|tab| &tab.value == selected));
        let children = tabs.into_iter().map(|tab| {
            let selected = selected.as_ref() == Some(&tab.value);
            let message = tab.enabled.then(|| on_select(tab.value));

            Pressable::new(
                column![text(tab.label).label(), Space::new().height(spacing::LG),]
                    .align_x(Alignment::Center),
            )
            .padding(Padding::ZERO.top(spacing::XS).horizontal(spacing::SM))
            .on_press_maybe(message)
            .style(move |theme, status| tab_style(theme, status, selected))
            .into()
        });

        container(stack![
            row(children).spacing(spacing::LG),
            canvas::Canvas::new(TabIndicator {
                selected_index,
                labels,
            })
            .width(Fill)
            .height(Fill),
        ])
        .center_x(Fill)
        .into()
    }
}

const INDICATOR_HEIGHT: f32 = 3.0;

struct TabIndicator<'a> {
    selected_index: Option<usize>,
    labels: Vec<&'a str>,
}

#[derive(Debug, Default)]
struct IndicatorState {
    animation: Option<Animation<f32>>,
    labels: Vec<String>,
}

impl IndicatorState {
    fn sync(&mut self, selected_index: Option<usize>, labels: &[&str], now: Instant) -> bool {
        if !self
            .labels
            .iter()
            .map(String::as_str)
            .eq(labels.iter().copied())
        {
            self.labels = labels.iter().map(|label| (*label).to_owned()).collect();
            self.animation = selected_index.map(|index| {
                Animation::new(index as f32)
                    .very_quick()
                    .easing(Easing::EaseOut)
            });
            return true;
        }

        let Some(selected_index) = selected_index else {
            return self.animation.take().is_some();
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

impl<Message> canvas::Program<Message> for TabIndicator<'_> {
    type State = IndicatorState;

    fn update(
        &self,
        state: &mut IndicatorState,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };

        if state.sync(self.selected_index, &self.labels, now) {
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
        if self.labels.is_empty() {
            return Vec::new();
        }

        let now = Instant::now();
        let center = state.animation.as_ref().map_or_else(
            || tab_center(renderer, &self.labels, selected_index),
            |animation| {
                animation.interpolate_with(
                    |index| tab_center(renderer, &self.labels, clamped(index, &self.labels)),
                    now,
                )
            },
        );
        let width = state.animation.as_ref().map_or_else(
            || tab_width(renderer, self.labels[selected_index]),
            |animation| {
                animation.interpolate_with(
                    |index| tab_width(renderer, self.labels[clamped(index, &self.labels)]),
                    now,
                )
            },
        );
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        frame.fill_rectangle(
            Point::new(center - width / 2.0, bounds.height - INDICATOR_HEIGHT),
            Size::new(width, INDICATOR_HEIGHT),
            theme.palette().primary,
        );

        vec![frame.into_geometry()]
    }
}

fn tab_center(renderer: &Renderer, labels: &[&str], index: usize) -> f32 {
    let index = index.min(labels.len() - 1);

    labels[..index]
        .iter()
        .map(|label| tab_width(renderer, label))
        .sum::<f32>()
        + spacing::LG * index as f32
        + tab_width(renderer, labels[index]) / 2.0
}

fn clamped(index: f32, labels: &[&str]) -> usize {
    (index as usize).min(labels.len() - 1)
}

fn tab_width(renderer: &Renderer, label: &str) -> f32 {
    label_width(renderer, label) + 2.0 * spacing::SM
}

fn label_width(renderer: &Renderer, label: &str) -> f32 {
    let paragraph =
        <Renderer as advanced_text::Renderer>::Paragraph::with_text(advanced_text::Text {
            content: label,
            bounds: Size::INFINITE,
            size: 18.0.into(),
            line_height: advanced_text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: advanced_text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: advanced_text::Shaping::default(),
            wrapping: advanced_text::Wrapping::default(),
        });

    paragraph.min_width()
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
        border: Border::default().rounded(6),
        ..button::Style::default()
    }
}
