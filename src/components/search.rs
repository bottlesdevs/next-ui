use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Padding, Pixels, Point,
    Rectangle, Shadow, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    widget::{Id, button, column, container, row, svg, text, text_input},
};

use crate::icons::Icon;

use super::{
    button::Button,
    pressable::{Pressable, Status},
    text::TextExt as _,
};

pub struct SearchResult<'a, Message> {
    key: String,
    title: &'a str,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
    on_select: Message,
    action: Option<(&'a str, Icon, Message)>,
}

impl<'a, Message> SearchResult<'a, Message> {
    pub fn new(key: impl Into<String>, title: &'a str, on_select: Message) -> Self {
        Self {
            key: key.into(),
            title,
            subtitle: None,
            icon: None,
            on_select,
            action: None,
        }
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn action(mut self, label: &'a str, icon: Icon, message: Message) -> Self {
        self.action = Some((label, icon, message));
        self
    }
}

pub enum SearchState<'a, Message> {
    Hidden,
    Loading,
    Results(Vec<SearchResult<'a, Message>>),
    Empty,
    Error(&'a str),
}

pub struct Search<'a, Message> {
    placeholder: &'a str,
    query: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    state: SearchState<'a, Message>,
    on_submit: Option<Message>,
    footer: Option<(&'a str, Message)>,
    id: Option<Id>,
    padding_x: f32,
    padding_y: f32,
}

impl<'a, Message> Search<'a, Message> {
    pub fn new(
        placeholder: &'a str,
        query: &'a str,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        Self {
            placeholder,
            query,
            on_input: Box::new(on_input),
            state: SearchState::Hidden,
            on_submit: None,
            footer: None,
            id: None,
            padding_x: 20.0,
            padding_y: 16.0,
        }
    }

    pub fn state(mut self, state: SearchState<'a, Message>) -> Self {
        self.state = state;
        self
    }

    pub fn on_submit(mut self, on_submit: Message) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    pub fn footer(mut self, label: &'a str, on_press: Message) -> Self {
        self.footer = Some((label, on_press));
        self
    }

    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn padding(mut self, padding: impl Into<Pixels>) -> Self {
        let padding = padding.into().0;
        self.padding_x = padding;
        self.padding_y = padding;
        self
    }

    pub fn padding_x(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_x = padding.into().0;
        self
    }

    pub fn padding_y(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_y = padding.into().0;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Search<'a, Message>> for Element<'a, Message> {
    fn from(search: Search<'a, Message>) -> Self {
        let mut input = text_input(search.placeholder, search.query)
            .on_input(search.on_input)
            .width(Fill)
            .padding(0)
            .size(18)
            .style(input_style);

        if let Some(id) = search.id {
            input = input.id(id);
        }

        let input = container(
            row![Icon::Search.view(), input]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding(Padding {
            top: search.padding_y,
            right: search.padding_x,
            bottom: search.padding_y,
            left: search.padding_x,
        })
        .style(search_style);
        let (panel, keys, selections) = panel(search.state, search.footer);

        Element::new(SearchWidget {
            input: input.into(),
            panel,
            keys,
            selections,
            query: search.query,
            on_submit: search.on_submit,
        })
    }
}

fn panel<'a, Message: Clone + 'a>(
    state: SearchState<'a, Message>,
    footer: Option<(&'a str, Message)>,
) -> (Panel<'a, Message>, Vec<String>, Vec<Message>) {
    let mut children = Vec::new();
    let mut keys = Vec::new();
    let mut selections = Vec::new();
    let visible = !matches!(state, SearchState::Hidden);
    let result_count = match state {
        SearchState::Results(results) => {
            for result in results {
                keys.push(result.key.clone());
                selections.push(result.on_select.clone());
                children.push(result_row(result));
            }

            children.len()
        }
        SearchState::Loading => {
            children.push(status_row("Searching…", None));
            0
        }
        SearchState::Empty => {
            children.push(status_row("No results", None));
            0
        }
        SearchState::Error(error) => {
            children.push(status_row(error, Some(crate::theme::ERROR)));
            0
        }
        SearchState::Hidden => 0,
    };

    if visible && let Some((label, message)) = footer {
        children.push(
            Pressable::new(
                row![text(label), Icon::Arrow.rotated(std::f32::consts::PI)]
                    .spacing(14)
                    .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([18, 20])
            .on_press(message)
            .style(footer_style)
            .into(),
        );
    }

    (
        Panel {
            children,
            result_count,
            highlighted: None,
            visible,
        },
        keys,
        selections,
    )
}

fn result_row<'a, Message: Clone + 'a>(result: SearchResult<'a, Message>) -> Element<'a, Message> {
    let mut labels = column![text(result.title).label()].spacing(4);

    if let Some(subtitle) = result.subtitle {
        labels = labels.push(text(subtitle).detail().muted());
    }

    let mut content = row![].spacing(14).align_y(Alignment::Center);

    if let Some(icon) = result.icon {
        content = content.push(
            svg(icon.handle())
                .width(20)
                .height(20)
                .content_fit(iced::ContentFit::Contain),
        );
    }

    content = content
        .push(labels)
        .push(iced::widget::Space::new().width(Fill));

    if let Some((label, icon, message)) = result.action {
        content = content.push(
            Button::new(label)
                .icon(icon)
                .pill()
                .surface()
                .on_press(message),
        );
    }

    Pressable::new(content)
        .width(Fill)
        .padding([14, 20])
        .on_press(result.on_select)
        .style(result_style)
        .into()
}

fn status_row<'a, Message: 'a>(label: &'a str, color: Option<Color>) -> Element<'a, Message> {
    container(text(label).label().style(move |theme: &Theme| text::Style {
        color: Some(color.unwrap_or(theme.extended_palette().secondary.weak.text)),
    }))
    .width(Fill)
    .padding(20)
    .into()
}

struct SearchWidget<'a, Message> {
    input: Element<'a, Message>,
    panel: Panel<'a, Message>,
    keys: Vec<String>,
    selections: Vec<Message>,
    query: &'a str,
    on_submit: Option<Message>,
}

#[derive(Debug, Default)]
struct SearchLocal {
    focused: bool,
    dismissed: bool,
    highlighted: Option<usize>,
    keys: Vec<String>,
    query: String,
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for SearchWidget<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SearchLocal>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SearchLocal::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.input),
            Tree::new(&self.panel as &dyn Widget<Message, Theme, iced::Renderer>),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.input);
        tree.children[1].diff(&self.panel as &dyn Widget<Message, Theme, iced::Renderer>);
        let state = tree.state.downcast_mut::<SearchLocal>();
        state.highlighted = preserve_highlight(&state.keys, state.highlighted, &self.keys);
        state.keys.clone_from(&self.keys);

        if state.query != self.query {
            state.query.clear();
            state.query.push_str(self.query);
            state.dismissed = false;
        }
    }

    fn size(&self) -> Size<Length> {
        self.input.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.input
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
        self.input
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
        let state = tree.state.downcast_mut::<SearchLocal>();

        if state.focused
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) = event
        {
            let result_count = self.selections.len();
            let handled = match key.as_ref() {
                keyboard::Key::Named(key::Named::ArrowDown) if result_count > 0 => {
                    state.highlighted = Some(
                        state
                            .highlighted
                            .map_or(0, |index| (index + 1) % result_count),
                    );
                    true
                }
                keyboard::Key::Named(key::Named::ArrowUp) if result_count > 0 => {
                    state.highlighted = Some(state.highlighted.map_or(result_count - 1, |index| {
                        (index + result_count - 1) % result_count
                    }));
                    true
                }
                keyboard::Key::Named(key::Named::Home) if result_count > 0 => {
                    state.highlighted = Some(0);
                    true
                }
                keyboard::Key::Named(key::Named::End) if result_count > 0 => {
                    state.highlighted = Some(result_count - 1);
                    true
                }
                keyboard::Key::Named(key::Named::Enter) => {
                    if let Some(index) = state.highlighted {
                        shell.publish(self.selections[index].clone());
                    } else if let Some(message) = &self.on_submit {
                        shell.publish(message.clone());
                    }
                    true
                }
                keyboard::Key::Named(key::Named::Escape) => {
                    state.dismissed = true;
                    true
                }
                _ => false,
            };

            if handled {
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        self.input.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let focused = has_focus(&mut self.input, &mut tree.children[0], layout, renderer);

        if focused && !state.focused {
            state.dismissed = false;
        }

        state.focused = focused;
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.input.as_widget().mouse_interaction(
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
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.input.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        _renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let (state, children) = (&mut tree.state, &mut tree.children);
        let state = state.downcast_mut::<SearchLocal>();

        if !self.panel.visible || !state.focused || state.dismissed {
            return None;
        }

        self.panel.highlighted = state.highlighted;
        let bounds = layout.bounds();

        Some(overlay::Element::new(Box::new(Anchored {
            position: bounds.position() + translation,
            target_height: bounds.height,
            width: bounds.width,
            viewport: *viewport,
            panel: &mut self.panel,
            tree: &mut children[1],
        })))
    }
}

fn has_focus<Message>(
    input: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
) -> bool {
    let mut count = operation::focusable::count();
    input.as_widget_mut().operate(
        tree,
        layout,
        renderer,
        &mut operation::black_box(&mut count),
    );

    matches!(
        Operation::finish(&count),
        operation::Outcome::Some(count) if count.focused.is_some()
    )
}

fn preserve_highlight(
    old_keys: &[String],
    old_highlight: Option<usize>,
    new_keys: &[String],
) -> Option<usize> {
    old_highlight
        .and_then(|index| old_keys.get(index))
        .and_then(|key| new_keys.iter().position(|candidate| candidate == key))
        .or((!new_keys.is_empty()).then_some(0))
}

struct Panel<'a, Message> {
    children: Vec<Element<'a, Message>>,
    result_count: usize,
    highlighted: Option<usize>,
    visible: bool,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Panel<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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
        let limits = limits.width(Fill).height(Length::Shrink);
        let mut y = 0.0;
        let mut children = Vec::with_capacity(self.children.len());

        for (child, tree) in self.children.iter_mut().zip(&mut tree.children) {
            let node = child
                .as_widget_mut()
                .layout(tree, renderer, &limits)
                .move_to(Point::new(0.0, y));
            y += node.size().height;
            children.push(node);
        }

        layout::Node::with_children(Size::new(limits.max().width, y), children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, tree), layout)| {
                    child
                        .as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                });
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
        self.children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .for_each(|((child, tree), layout)| {
                child.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                );
            });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border::default().rounded(12),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(theme.extended_palette().background.neutral.color),
        );
        let layouts: Vec<_> = layout.children().collect();

        if let Some(index) = self.highlighted.filter(|index| *index < self.result_count) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layouts[index].bounds(),
                    border: Border::default().rounded(10),
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(theme.extended_palette().background.stronger.color),
            );
        }

        self.children
            .iter()
            .zip(&tree.children)
            .zip(layouts)
            .for_each(|((child, tree), layout)| {
                child
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, viewport);
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

struct Anchored<'a, 'b, Message>
where
    'b: 'a,
{
    position: Point,
    target_height: f32,
    width: f32,
    viewport: Rectangle,
    panel: &'a mut Panel<'b, Message>,
    tree: &'a mut Tree,
}

impl<Message> iced::advanced::Overlay<Message, Theme, iced::Renderer>
    for Anchored<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let below = bounds.height - (self.position.y + self.target_height + 6.0);
        let above = self.position.y - 6.0;
        let max_height = below.max(above).max(0.0);
        let limits = layout::Limits::new(
            Size::new(self.width, 0.0),
            Size::new(self.width, max_height),
        );
        let node = self.panel.layout(self.tree, renderer, &limits);
        let height = node.size().height;

        node.move_to(if below >= height || below >= above {
            self.position + Vector::new(0.0, self.target_height + 6.0)
        } else {
            self.position - Vector::new(0.0, height + 6.0)
        })
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.panel.update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.panel
            .mouse_interaction(self.tree, layout, cursor, &self.viewport, renderer)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.panel.draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &self.viewport,
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.panel.operate(self.tree, layout, renderer, operation);
    }
}

fn search_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.neutral.color,
        )),
        border: Border::default().rounded(8),
        ..container::Style::default()
    }
}

fn input_style(theme: &Theme, _: text_input::Status) -> text_input::Style {
    let colors = theme.extended_palette();

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: colors.secondary.weak.text,
        placeholder: colors.secondary.weak.text,
        value: theme.palette().text,
        selection: theme.palette().primary,
    }
}

fn result_style(theme: &Theme, status: Status) -> button::Style {
    let highlighted = matches!(status, Status::Hovered | Status::Pressed | Status::Focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(10),
        ..button::Style::default()
    }
}

fn footer_style(theme: &Theme, status: Status) -> button::Style {
    let colors = if matches!(status, Status::Hovered | Status::Pressed) {
        theme.extended_palette().background.strongest
    } else {
        theme.extended_palette().background.stronger
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: theme.extended_palette().secondary.weak.text,
        border: Border::default().rounded(iced::border::bottom(12)),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::preserve_highlight;

    #[test]
    fn keyboard_highlight_follows_stable_result_keys() {
        let old = vec!["one".to_owned(), "two".to_owned()];
        let new = vec!["two".to_owned(), "three".to_owned()];

        assert_eq!(preserve_highlight(&old, Some(1), &new), Some(0));
        assert_eq!(preserve_highlight(&old, Some(0), &new), Some(0));
        assert_eq!(preserve_highlight(&old, Some(0), &[]), None);
    }
}
