use std::{cell::Cell, collections::HashSet, rc::Rc};

use iced::{
    Alignment, Element, Event, Fill, Length, Point, Rectangle, Size, Theme,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        widget::{Id, Operation, Tree, tree},
    },
    animation::{Animation, Easing},
    keyboard::{self, key},
    time::Instant,
    touch,
    widget::{column, container, row, scrollable, text},
    window,
};

use crate::{icons::Icon, theme};

use super::{
    control::{ActionSurface, ActivationSignal, descendant_is_focused},
    draw_caret,
    list_navigation::{Navigation, navigate_index, reconcile_index, reveal},
    list_row::{self, ListRow},
    menu, spacing,
    surface::{Surface, SurfaceRole},
    text::TextExt as _,
};

const PANEL_PADDING: [f32; 2] = [spacing::MD, spacing::SM];

/// A single-Tab-stop row that owns an in-flow option disclosure and keyboard
/// navigation.
pub struct SelectorRow<'a, T, Message> {
    title: &'a str,
    options: &'a [T],
    selected: Option<&'a T>,
    on_selected: Option<Box<dyn Fn(T) -> Message + 'a>>,
    placeholder: &'a str,
    label: Box<dyn Fn(&T) -> String + 'a>,
    key: Box<dyn Fn(&T) -> String + 'a>,
    icon: Option<Icon>,
}

impl<'a, T: ToString, Message> SelectorRow<'a, T, Message> {
    pub fn new(
        title: &'a str,
        options: &'a [T],
        selected: Option<&'a T>,
        key: impl Fn(&T) -> String + 'a,
    ) -> Self {
        Self {
            title,
            options,
            selected,
            on_selected: None,
            placeholder: "",
            label: Box::new(ToString::to_string),
            key: Box::new(key),
            icon: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn label(mut self, label: impl Fn(&T) -> String + 'a) -> Self {
        self.label = Box::new(label);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_selected(mut self, on_selected: impl Fn(T) -> Message + 'a) -> Self {
        self.on_selected = Some(Box::new(on_selected));
        self
    }

    pub fn on_selected_maybe(mut self, on_selected: Option<impl Fn(T) -> Message + 'a>) -> Self {
        self.on_selected = on_selected.map(|on_selected| Box::new(on_selected) as _);
        self
    }
}

impl<'a, T, Message> From<SelectorRow<'a, T, Message>> for Element<'a, Message>
where
    T: Clone + PartialEq + ToString + 'a,
    Message: Clone + 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let SelectorRow {
            title,
            options,
            selected,
            on_selected,
            placeholder,
            label,
            key,
            icon,
        } = selector;
        let selected =
            selected.and_then(|selected| options.iter().position(|option| option == selected));
        let labels: Vec<_> = options.iter().map(label).collect();
        let keys: Vec<_> = options.iter().map(key).collect();
        let mut unique = HashSet::with_capacity(keys.len());
        assert!(
            keys.iter().all(|key| unique.insert(key.as_str())),
            "SelectorRow keys must be unique"
        );

        let value = selected
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| placeholder.to_owned());
        let on_selected = on_selected.map(|on_selected| {
            Box::new(move |index: usize| on_selected(options[index].clone()))
                as Box<dyn Fn(usize) -> Message>
        });
        let enabled = on_selected.is_some() && !keys.is_empty();
        let activated = Rc::new(Cell::new(false));
        let expanded = Rc::new(Cell::new(false));
        let highlighted = Rc::new(Cell::new(selected.or((!keys.is_empty()).then_some(0))));
        let header = ActionSurface::new(header(title, value, icon))
            .width(Fill)
            .sensitive(enabled)
            .activation_signal(activated.clone())
            .style({
                let expanded = expanded.clone();
                move |theme, mut state| {
                    state.expanded = expanded.get();
                    list_row::style(theme, state)
                }
            });

        let mut activations = Vec::with_capacity(keys.len());
        let rows = labels
            .into_iter()
            .zip(keys.iter().cloned())
            .enumerate()
            .map(|(index, (label, key))| {
                let activation = Rc::new(Cell::new(false));
                activations.push(activation.clone());
                let highlighted = highlighted.clone();
                let row: Element<'a, Message> = container(
                    ActionSurface::new(container(text(label).label()).width(Fill))
                        .id(key.into())
                        .width(Fill)
                        .padding([spacing::SM, spacing::SM])
                        .activation_signal(activation)
                        .selected(selected == Some(index))
                        .style(move |theme, mut state| {
                            state.keyboard_highlighted = highlighted.get() == Some(index);
                            menu::row_style(theme, state)
                        }),
                )
                .width(Fill)
                .into();
                row
            });
        let panel = Surface::new(
            SurfaceRole::Panel,
            scrollable(
                container(column(rows).width(Fill))
                    .width(Fill)
                    .padding(PANEL_PADDING),
            )
            .width(Fill)
            .style(theme::scrollbar),
        );

        Element::new(Selector {
            header: header.into(),
            panel: panel.into(),
            selected,
            keys,
            activations,
            on_selected,
            activated,
            expanded,
            highlighted,
        })
    }
}

fn header<'a, Message: 'a>(
    title: &'a str,
    value: String,
    icon: Option<Icon>,
) -> Element<'a, Message> {
    let mut value_row = row![].spacing(spacing::SM).align_y(Alignment::Center);

    if let Some(icon) = icon {
        value_row = value_row.push(
            icon.view()
                .width(list_row::BODY_SIZE)
                .height(list_row::BODY_SIZE),
        );
    }

    value_row = value_row.push(text(value).size(list_row::BODY_SIZE).muted());

    ListRow::new(
        column![text(title).label().medium(), value_row]
            .width(Fill)
            .spacing(spacing::XS),
    )
    .into_disclosure_content()
    .element
}

struct Selector<'a, Message> {
    header: Element<'a, Message>,
    panel: Element<'a, Message>,
    selected: Option<usize>,
    keys: Vec<String>,
    activations: Vec<ActivationSignal>,
    on_selected: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    activated: ActivationSignal,
    expanded: Rc<Cell<bool>>,
    highlighted: Rc<Cell<Option<usize>>>,
}

impl<Message> Selector<'_, Message> {
    fn is_enabled(&self) -> bool {
        self.on_selected.is_some() && !self.keys.is_empty()
    }

    fn select(&self, index: usize) -> Option<Message> {
        self.on_selected
            .as_ref()
            .map(|on_selected| on_selected(index))
    }

    fn take_activation(&self) -> Option<usize> {
        self.activations
            .iter()
            .position(|activation| activation.replace(false))
    }
}

#[derive(Debug)]
struct State {
    expansion: Animation<bool>,
    highlighted: Option<usize>,
    keys: Vec<String>,
    reveal: Option<Id>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            expansion: Animation::new(false).very_quick().easing(Easing::EaseOut),
            highlighted: None,
            keys: Vec::new(),
            reveal: None,
        }
    }
}

impl State {
    fn is_open(&self) -> bool {
        self.expansion.value()
    }

    fn set_open(&mut self, open: bool, now: Instant) {
        if self.is_open() != open {
            self.expansion.go_mut(open, now);
        }
    }

    fn expansion(&self, now: Instant) -> f32 {
        self.expansion.interpolate(0.0, 1.0, now)
    }
}

fn navigate(
    open: bool,
    current: Option<usize>,
    selected: Option<usize>,
    count: usize,
    navigation: Navigation,
) -> Option<usize> {
    let last = count.checked_sub(1)?;

    if open {
        navigate_index(current, count, navigation, false)
    } else {
        Some(match navigation {
            Navigation::Next => selected.unwrap_or(0),
            Navigation::Previous => selected.unwrap_or(last),
            Navigation::First => 0,
            Navigation::Last => last,
        })
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Selector<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let mut state = State::default();
        state.keys.clone_from(&self.keys);
        if self.is_enabled() {
            state.highlighted = self.selected.or(Some(0));
        }
        tree::State::new(state)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.header), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        let reset_panel = tree.state.downcast_ref::<State>().keys != self.keys;
        if tree.children.len() != 2 {
            tree.children = self.children();
        } else {
            tree.children[0].diff(&self.header);
            if reset_panel {
                tree.children[1] = Tree::new(&self.panel);
            } else {
                tree.children[1].diff(&self.panel);
            }
        }

        let state = tree.state.downcast_mut::<State>();
        let previous_highlight = state.highlighted;
        let highlighted = reconcile_index(&state.keys, state.highlighted, &self.keys);
        state.keys.clone_from(&self.keys);
        if self.is_enabled() {
            state.highlighted = highlighted.or(self.selected).or(Some(0));
        } else {
            state.set_open(false, Instant::now());
            state.highlighted = None;
        }

        if state.is_open()
            && state.highlighted != previous_highlight
            && let Some(index) = state.highlighted
        {
            state.reveal = Some(self.keys[index].clone().into());
        }

        self.expanded.set(state.is_open());
        self.highlighted.set(state.highlighted);
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
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let header = self
            .header
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let header_height = header.size().height;
        let width = header.size().width;
        let panel_limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, (limits.max().height - header_height).max(0.0)),
        );
        let panel = self
            .panel
            .as_widget_mut()
            .layout(&mut tree.children[1], renderer, &panel_limits)
            .move_to(Point::new(0.0, header_height));
        let expansion = tree.state.downcast_ref::<State>().expansion(Instant::now());

        layout::Node::with_children(
            Size::new(
                width,
                disclosed_height(header_height, panel.size().height, expansion),
            ),
            vec![header, panel],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let (header_layout, _) = layout_parts(layout);
        self.header.as_widget_mut().operate(
            &mut tree.children[0],
            header_layout,
            renderer,
            operation,
        );
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
        let (state, children) = (&mut tree.state, &mut tree.children);
        let state = state.downcast_mut::<State>();
        let (header_layout, panel_layout) = layout_parts(layout);

        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && state.expansion.is_animating(*now)
        {
            shell.invalidate_layout();
            shell.request_redraw();
        }

        let header_focused = descendant_is_focused(
            self.header.as_widget_mut(),
            &mut children[0],
            header_layout,
            renderer,
        );

        if self.is_enabled()
            && header_focused
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) = event
        {
            if state.is_open() && matches!(key.as_ref(), keyboard::Key::Named(key::Named::Tab)) {
                state.set_open(false, Instant::now());
                self.expanded.set(false);
                shell.invalidate_layout();
                shell.request_redraw();
                return;
            }

            let navigation = match key.as_ref() {
                keyboard::Key::Named(key::Named::ArrowDown) => Some(Navigation::Next),
                keyboard::Key::Named(key::Named::ArrowUp) => Some(Navigation::Previous),
                keyboard::Key::Named(key::Named::Home) => Some(Navigation::First),
                keyboard::Key::Named(key::Named::End) => Some(Navigation::Last),
                _ => None,
            };

            if let Some(navigation) = navigation {
                state.highlighted = navigate(
                    state.is_open(),
                    state.highlighted,
                    self.selected,
                    self.keys.len(),
                    navigation,
                );
                state.set_open(true, Instant::now());
                if let Some(index) = state.highlighted {
                    state.reveal = Some(self.keys[index].clone().into());
                }
                self.expanded.set(true);
                self.highlighted.set(state.highlighted);
                shell.capture_event();
                shell.invalidate_layout();
                shell.request_redraw();
                return;
            }

            if state.is_open()
                && matches!(
                    key.as_ref(),
                    keyboard::Key::Named(key::Named::Enter | key::Named::Space)
                )
                && let Some(index) = state.highlighted
                && let Some(message) = self.select(index)
            {
                state.set_open(false, Instant::now());
                self.expanded.set(false);
                shell.publish(message);
                shell.capture_event();
                shell.invalidate_layout();
                shell.request_redraw();
                return;
            }

            if state.is_open() && matches!(key.as_ref(), keyboard::Key::Named(key::Named::Escape)) {
                state.set_open(false, Instant::now());
                self.expanded.set(false);
                shell.capture_event();
                shell.invalidate_layout();
                shell.request_redraw();
                return;
            }
        }

        self.header.as_widget_mut().update(
            &mut children[0],
            event,
            header_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if state.is_open() {
            let panel_viewport = layout.bounds().intersection(viewport).unwrap_or_default();
            self.panel.as_widget_mut().update(
                &mut children[1],
                event,
                panel_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                &panel_viewport,
            );

            if let Some(id) = state.reveal.take()
                && reveal(
                    self.panel.as_widget_mut(),
                    &mut children[1],
                    panel_layout,
                    renderer,
                    &id,
                )
            {
                shell.request_redraw();
            }

            if let Some(index) = self.take_activation()
                && let Some(message) = self.select(index)
            {
                state.highlighted = Some(index);
                state.set_open(false, Instant::now());
                self.expanded.set(false);
                self.highlighted.set(state.highlighted);
                shell.publish(message);
                shell.invalidate_layout();
                shell.request_redraw();
                return;
            }
        }

        if self.activated.replace(false) {
            state.set_open(!state.is_open(), Instant::now());
            if state.is_open() {
                state.highlighted = self.selected.or(Some(0));
                if let Some(index) = state.highlighted {
                    state.reveal = Some(self.keys[index].clone().into());
                }
            }
            self.expanded.set(state.is_open());
            self.highlighted.set(state.highlighted);
            shell.invalidate_layout();
            shell.request_redraw();
        } else if state.is_open() && outside_press(event, cursor, layout.bounds()) {
            state.set_open(false, Instant::now());
            self.expanded.set(false);
            shell.invalidate_layout();
            shell.request_redraw();
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
        let state = tree.state.downcast_ref::<State>();
        let (header_layout, panel_layout) = layout_parts(layout);
        let header = self.header.as_widget().mouse_interaction(
            &tree.children[0],
            header_layout,
            cursor,
            viewport,
            renderer,
        );

        if state.is_open() {
            header.max(self.panel.as_widget().mouse_interaction(
                &tree.children[1],
                panel_layout,
                cursor,
                viewport,
                renderer,
            ))
        } else {
            header
        }
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
        let state = tree.state.downcast_ref::<State>();
        let (header_layout, panel_layout) = layout_parts(layout);
        let expansion = state.expansion(Instant::now());
        self.expanded.set(expansion > 0.0);

        if expansion > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: iced::Border::default().rounded(6),
                    snap: true,
                    ..renderer::Quad::default()
                },
                iced::Background::Color(crate::theme::colors(theme).border),
            );
        }

        self.header.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            header_layout,
            cursor,
            viewport,
        );

        let content = header_layout
            .children()
            .next()
            .expect("selector action content");
        let row = content.children().next().expect("selector header row");
        let caret_slot = row.children().nth(1).expect("selector caret slot");
        draw_caret(renderer, theme, caret_slot.bounds(), expansion);

        if expansion > 0.0
            && let Some(clip) = layout.bounds().intersection(viewport)
        {
            renderer.with_layer(clip, |renderer| {
                self.panel.as_widget().draw(
                    &tree.children[1],
                    renderer,
                    theme,
                    style,
                    panel_layout,
                    cursor,
                    &clip,
                );
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            y: panel_layout.bounds().y,
                            height: 1.0,
                            ..layout.bounds()
                        },
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    iced::Background::Color(crate::theme::colors(theme).selection),
                );
            });
        }
    }
}

fn layout_parts(layout: Layout<'_>) -> (Layout<'_>, Layout<'_>) {
    let mut children = layout.children();
    (
        children.next().expect("selector header"),
        children.next().expect("selector panel"),
    )
}

fn outside_press(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
            .position()
            .is_some_and(|position| !bounds.contains(position)),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        _ => false,
    }
}

fn disclosed_height(header: f32, panel: f32, expansion: f32) -> f32 {
    header + panel * expansion.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "SelectorRow keys must be unique")]
    fn duplicate_keys_are_rejected() {
        let options = ["first", "second"];
        let _: Element<'_, ()> =
            SelectorRow::new("Duplicate", &options, None, |_| "same".into()).into();
    }
}
