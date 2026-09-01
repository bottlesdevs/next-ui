use iced::advanced::widget::{Id, Operation, Tree, operation};
use iced::advanced::{Layout, Widget};
use iced::{Rectangle, Theme, Vector};

pub(super) fn reconcile_index<T: PartialEq>(
    old_keys: &[T],
    old_index: Option<usize>,
    new_keys: &[T],
) -> Option<usize> {
    old_index
        .and_then(|index| old_keys.get(index))
        .and_then(|key| new_keys.iter().position(|candidate| candidate == key))
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Navigation {
    Next,
    Previous,
    First,
    Last,
}

pub(super) fn navigate_index(
    current: Option<usize>,
    count: usize,
    navigation: Navigation,
    wrap: bool,
) -> Option<usize> {
    let last = count.checked_sub(1)?;

    Some(match navigation {
        Navigation::Next if wrap => current.map_or(0, |index| (index + 1) % count),
        Navigation::Previous if wrap => current.map_or(last, |index| (index + count - 1) % count),
        Navigation::Next => current.map_or(0, |index| (index + 1).min(last)),
        Navigation::Previous => current.unwrap_or(last).saturating_sub(1),
        Navigation::First => 0,
        Navigation::Last => last,
    })
}

pub(super) fn reveal<Message>(
    content: &mut dyn Widget<Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    marker: &Id,
) -> bool {
    let mut find = FindTargetBounds {
        target: marker.clone(),
        bounds: None,
    };
    content.operate(tree, layout, renderer, &mut find);
    let Some(bounds) = find.bounds else {
        return false;
    };
    let mut reveal = RevealBounds {
        bounds,
        changed: false,
    };
    content.operate(tree, layout, renderer, &mut reveal);
    reveal.changed
}

struct FindTargetBounds {
    target: Id,
    bounds: Option<Rectangle>,
}

impl<Message> Operation<Message> for FindTargetBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.bounds = Some(bounds);
        }
    }
}

struct RevealBounds {
    bounds: Rectangle,
    changed: bool,
}

impl<Message> Operation<Message> for RevealBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        if !content_bounds.contains(self.bounds.center()) {
            return;
        }

        let target = Rectangle {
            x: self.bounds.x - translation.x,
            y: self.bounds.y - translation.y,
            ..self.bounds
        };
        let x = if target.x < bounds.x {
            target.x - bounds.x
        } else if target.x + target.width > bounds.x + bounds.width {
            target.x + target.width - (bounds.x + bounds.width)
        } else {
            0.0
        };
        let y = if target.y < bounds.y {
            target.y - bounds.y
        } else if target.y + target.height > bounds.y + bounds.height {
            target.y + target.height - (bounds.y + bounds.height)
        } else {
            0.0
        };
        let delta = Vector::new(x, y);

        if delta != Vector::ZERO {
            state.scroll_by(
                operation::scrollable::AbsoluteOffset {
                    x: delta.x,
                    y: delta.y,
                },
                bounds,
                content_bounds,
            );
            self.changed = true;
        }
    }
}
