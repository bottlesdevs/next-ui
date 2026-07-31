use iced::{
    Alignment, Border, Element, Event, Fill, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation},
    },
    keyboard::{self, key},
    widget::{button, column, row, rule, text},
};

use super::{
    pressable::{Pressable, Status},
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
    T: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(tabs: Tabs<'a, T, Message>) -> Self {
        let mut messages = Vec::new();
        let children = tabs.tabs.into_iter().map(|tab| {
            let selected = tabs.selected.as_ref() == Some(&tab.value);
            let message = tab.enabled.then(|| (tabs.on_select)(tab.value));

            if let Some(message) = &message {
                messages.push(message.clone());
            }

            Pressable::new(
                column![
                    text(tab.label).label(),
                    rule::horizontal(3).style(move |theme: &Theme| rule::Style {
                        color: if selected {
                            theme.palette().primary
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        ..rule::default(theme)
                    }),
                ]
                .align_x(Alignment::Center)
                .spacing(10),
            )
            .width(Fill)
            .padding([8, 16])
            .on_press_maybe(message)
            .style(move |theme, status| tab_style(theme, status, selected))
            .into()
        });

        Element::new(TabList {
            content: row(children).width(Fill).into(),
            messages,
        })
    }
}

struct TabList<'a, Message> {
    content: Element<'a, Message>,
    messages: Vec<Message>,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for TabList<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() || self.messages.is_empty() {
            return;
        }

        let Event::Keyboard(keyboard::Event::KeyPressed {
            key, repeat: false, ..
        }) = event
        else {
            return;
        };
        let Some(focused) = focused_tab(&mut self.content, tree, layout, renderer) else {
            return;
        };
        let last = self.messages.len() - 1;
        let target = match key.as_ref() {
            keyboard::Key::Named(key::Named::ArrowRight) => (focused + 1) % self.messages.len(),
            keyboard::Key::Named(key::Named::ArrowLeft) => {
                (focused + self.messages.len() - 1) % self.messages.len()
            }
            keyboard::Key::Named(key::Named::Home) => 0,
            keyboard::Key::Named(key::Named::End) => last,
            _ => return,
        };

        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut FocusNth { target, current: 0 },
        );
        shell.publish(self.messages[target].clone());
        shell.request_redraw();
        shell.capture_event();
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
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
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn focused_tab<Message>(
    content: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> Option<usize> {
    let mut count = operation::focusable::count();
    content.as_widget_mut().operate(
        &mut tree.children[0],
        layout,
        renderer,
        &mut operation::black_box(&mut count),
    );

    match Operation::finish(&count) {
        operation::Outcome::Some(count) => count.focused,
        operation::Outcome::None | operation::Outcome::Chain(_) => None,
    }
}

struct FocusNth {
    target: usize,
    current: usize,
}

impl Operation for FocusNth {
    fn focusable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if self.current == self.target {
            state.focus();
        } else {
            state.unfocus();
        }

        self.current += 1;
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
}

fn tab_style(theme: &Theme, status: Status, selected: bool) -> button::Style {
    button::Style {
        text_color: if selected
            || matches!(status, Status::Hovered | Status::Pressed | Status::Focused)
        {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default()
            .rounded(4)
            .color(if status == Status::Focused {
                theme.palette().primary
            } else {
                iced::Color::TRANSPARENT
            })
            .width(if status == Status::Focused { 2 } else { 0 }),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Tab, Tabs};

    #[test]
    fn selection_uses_stable_values_and_may_be_missing() {
        let tabs = Tabs::<_, ()>::new(
            [
                Tab::new("bottles", "Bottles"),
                Tab::new("library", "Library"),
            ],
            Some("removed"),
            |_| (),
        );

        assert!(
            !tabs
                .tabs
                .iter()
                .any(|tab| Some(&tab.value) == tabs.selected.as_ref())
        );
    }
}
