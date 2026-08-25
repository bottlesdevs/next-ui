use std::{cell::Cell, rc::Rc};

use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Point, Rectangle, Shadow,
    Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, operation, tree},
    },
    keyboard::{self, key},
    widget::{Id, button, column, container, row, scrollable, svg, text, text_input},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    pressable::{Pressable, Status},
    spacing,
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
                .spacing(spacing::SM)
                .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([spacing::SM, spacing::MD])
        .style(search_style);
        let (panel_body, panel_footer, visible, keys, selections, highlight) =
            panel(search.state, search.footer);

        Element::new(SearchWidget {
            input: input.into(),
            panel_body,
            panel_footer,
            visible,
            keys,
            selections,
            highlight,
            query: search.query,
            on_submit: search.on_submit,
        })
    }
}

fn panel<'a, Message: Clone + 'a>(
    state: SearchState<'a, Message>,
    footer: Option<(&'a str, Message)>,
) -> (
    Element<'a, Message>,
    Option<Element<'a, Message>>,
    bool,
    Vec<String>,
    Vec<Message>,
    Rc<Cell<Option<usize>>>,
) {
    let mut keys = Vec::new();
    let mut selections = Vec::new();
    let highlight = Rc::new(Cell::new(None));
    let visible = !matches!(state, SearchState::Hidden);
    let body: Element<'a, Message> = match state {
        SearchState::Results(results) => {
            let mut rows = column![].width(Fill);

            for (index, result) in results.into_iter().enumerate() {
                keys.push(result.key.clone());
                selections.push(result.on_select.clone());
                rows = rows.push(result_row(result, Rc::clone(&highlight), index));
            }

            container(rows).width(Fill).padding(spacing::MD).into()
        }
        SearchState::Loading => status_row("Searching…", None),
        SearchState::Empty => status_row("No results", None),
        SearchState::Error(error) => status_row(error, Some(crate::theme::ERROR)),
        SearchState::Hidden => column![].into(),
    };

    let footer = if visible {
        footer.map(|(label, message)| {
            Pressable::new(
                row![text(label), Icon::Arrow.rotated(std::f32::consts::PI)]
                    .spacing(spacing::SM)
                    .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding(spacing::MD)
            .on_press(message)
            .style(footer_style)
            .into()
        })
    } else {
        None
    };

    (
        scrollable(body).width(Fill).into(),
        footer,
        visible,
        keys,
        selections,
        highlight,
    )
}

fn result_row<'a, Message: Clone + 'a>(
    result: SearchResult<'a, Message>,
    highlight: Rc<Cell<Option<usize>>>,
    index: usize,
) -> Element<'a, Message> {
    let mut labels = column![text(result.title).label()].spacing(spacing::XS);

    if let Some(subtitle) = result.subtitle {
        labels = labels.push(text(subtitle).detail().muted());
    }

    let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);

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
                .trailing_icon(icon)
                .icon_rotation(if icon == Icon::Arrow {
                    std::f32::consts::PI
                } else {
                    0.0
                })
                .kind(ButtonKind::Surface)
                .on_press(message),
        );
    }

    Pressable::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press(result.on_select)
        .style(move |theme, status| result_style(theme, status, highlight.get() == Some(index)))
        .into()
}

fn status_row<'a, Message: 'a>(label: &'a str, color: Option<Color>) -> Element<'a, Message> {
    container(text(label).label().style(move |theme: &Theme| text::Style {
        color: Some(color.unwrap_or(theme.extended_palette().secondary.weak.text)),
    }))
    .width(Fill)
    .padding(spacing::MD)
    .into()
}

struct SearchWidget<'a, Message> {
    input: Element<'a, Message>,
    panel_body: Element<'a, Message>,
    panel_footer: Option<Element<'a, Message>>,
    visible: bool,
    keys: Vec<String>,
    selections: Vec<Message>,
    highlight: Rc<Cell<Option<usize>>>,
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
        let mut children = vec![Tree::new(&self.input), Tree::new(&self.panel_body)];

        if let Some(footer) = &self.panel_footer {
            children.push(Tree::new(footer));
        }

        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut children: Vec<&dyn Widget<Message, Theme, iced::Renderer>> =
            vec![self.input.as_widget(), self.panel_body.as_widget()];

        if let Some(footer) = &self.panel_footer {
            children.push(footer.as_widget());
        }

        tree.diff_children(&children);
        let state = tree.state.downcast_mut::<SearchLocal>();
        state.highlighted = preserve_highlight(&state.keys, state.highlighted, &self.keys);
        state.keys.clone_from(&self.keys);
        self.highlight.set(state.highlighted);

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
                self.highlight.set(state.highlighted);
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

        if !self.visible || !state.focused || state.dismissed {
            return None;
        }

        self.highlight.set(state.highlighted);
        let bounds = layout.bounds();
        let (_, panel_trees) = children.split_at_mut(1);
        let (body_trees, footer_trees) = panel_trees.split_at_mut(1);

        Some(overlay::Element::new(Box::new(Anchored {
            position: bounds.position() + translation,
            target_height: bounds.height,
            width: bounds.width,
            viewport: *viewport,
            body: &mut self.panel_body,
            body_tree: &mut body_trees[0],
            footer: self.panel_footer.as_mut().zip(footer_trees.first_mut()),
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

#[cfg(test)]
mod tests {
    use super::preserve_highlight;

    #[test]
    fn preserves_highlight_across_result_changes() {
        let old = ["a", "b", "c"].map(String::from);

        assert_eq!(
            preserve_highlight(&old, Some(1), &["c", "a", "b"].map(String::from),),
            Some(2)
        );
        assert_eq!(
            preserve_highlight(&old, Some(1), &["a", "c"].map(String::from)),
            Some(0)
        );
        assert_eq!(preserve_highlight(&old, Some(1), &[]), None);
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
    body: &'a mut Element<'b, Message>,
    body_tree: &'a mut Tree,
    footer: Option<(&'a mut Element<'b, Message>, &'a mut Tree)>,
}

impl<Message> iced::advanced::Overlay<Message, Theme, iced::Renderer>
    for Anchored<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let below = bounds.height - (self.position.y + self.target_height + spacing::XS);
        let above = self.position.y - spacing::XS;
        let max_height = below.max(above).max(0.0);
        let limits = layout::Limits::new(
            Size::new(self.width, 0.0),
            Size::new(self.width, max_height),
        );
        let footer = self
            .footer
            .as_mut()
            .map(|(footer, tree)| footer.as_widget_mut().layout(tree, renderer, &limits));
        let footer_height = footer.as_ref().map_or(0.0, |node| node.size().height);
        let body_limits = layout::Limits::new(
            Size::new(self.width, 0.0),
            Size::new(self.width, (max_height - footer_height).max(0.0)),
        );
        let body = self
            .body
            .as_widget_mut()
            .layout(self.body_tree, renderer, &body_limits);
        let body_height = body.size().height;
        let height = body_height + footer_height;
        let mut children = vec![body];

        if let Some(footer) = footer {
            children.push(footer.move_to(Point::new(0.0, body_height)));
        }

        layout::Node::with_children(Size::new(self.width, height), children).move_to(
            if below >= height || below >= above {
                self.position + Vector::new(0.0, self.target_height + spacing::XS)
            } else {
                self.position - Vector::new(0.0, height + spacing::XS)
            },
        )
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
        let mut children = layout.children();
        self.body.as_widget_mut().update(
            self.body_tree,
            event,
            children.next().expect("search panel body layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );

        if let Some(((footer, tree), footer_layout)) = self.footer.as_mut().zip(children.next()) {
            footer.as_widget_mut().update(
                tree,
                event,
                footer_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                &self.viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut children = layout.children();
        let body = self.body.as_widget().mouse_interaction(
            self.body_tree,
            children.next().expect("search panel body layout"),
            cursor,
            &self.viewport,
            renderer,
        );

        self.footer
            .as_ref()
            .zip(children.next())
            .map_or(body, |((footer, tree), footer_layout)| {
                body.max(footer.as_widget().mouse_interaction(
                    tree,
                    footer_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                ))
            })
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: Border::default().rounded(6),
                shadow: Shadow::default(),
                snap: true,
            },
            Background::Color(theme.extended_palette().background.neutral.color),
        );

        let mut children = layout.children();
        self.body.as_widget().draw(
            self.body_tree,
            renderer,
            theme,
            style,
            children.next().expect("search panel body layout"),
            cursor,
            &self.viewport,
        );

        if let Some(((footer, tree), footer_layout)) = self.footer.as_ref().zip(children.next()) {
            footer.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                footer_layout,
                cursor,
                &self.viewport,
            );
        }
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            let mut children = layout.children();
            self.body.as_widget_mut().operate(
                self.body_tree,
                children.next().expect("search panel body layout"),
                renderer,
                operation,
            );

            if let Some(((footer, tree), footer_layout)) = self.footer.as_mut().zip(children.next())
            {
                footer
                    .as_widget_mut()
                    .operate(tree, footer_layout, renderer, operation);
            }
        });
    }
}

fn search_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.neutral.color,
        )),
        border: Border::default().rounded(6),
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

fn result_style(theme: &Theme, status: Status, keyboard_highlighted: bool) -> button::Style {
    let highlighted = keyboard_highlighted
        || matches!(status, Status::Hovered | Status::Pressed | Status::Focused);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.weak.text
        },
        border: Border::default().rounded(6),
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
        border: Border::default().rounded(iced::border::bottom(6)),
        ..button::Style::default()
    }
}
