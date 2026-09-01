use std::{cell::Cell, collections::HashSet, rc::Rc};

use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Rectangle, Size, Theme,
    Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
    keyboard::{self, key},
    widget::{Id, column, container, row, scrollable, text, text_input},
};

use crate::icons::Icon;

use super::{
    anchored_overlay::{DismissOutcome, DismissReason, PopupCore, PopupFocus, PopupFocusState},
    button::{Button, ButtonKind},
    control::{descendant_is_focused, event_cursor, focus_first_descendant, unfocus_descendants},
    list_navigation::{Navigation, navigate_index, reconcile_index, reveal},
    menu::{footer as panel_footer, item as menu_item, row_content},
    spacing,
    surface::{SurfaceRole, scoped_overlay},
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
    /// Creates a result whose key preserves highlighting across result changes.
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
            .style(|theme, _| input_style(theme));

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
        let PanelParts {
            panel,
            visible,
            keys,
            selections,
            highlight,
            focus_targets,
        } = panel(search.state, search.footer);

        Element::new(SearchWidget {
            input: input.into(),
            panel,
            visible,
            keys,
            selections,
            highlight,
            query: search.query,
            on_submit: search.on_submit,
            focus_targets,
        })
    }
}

struct PanelParts<'a, Message> {
    panel: SearchPanel<'a, Message>,
    visible: bool,
    keys: Vec<String>,
    selections: Vec<Message>,
    highlight: Rc<Cell<Option<usize>>>,
    focus_targets: Vec<Id>,
}

fn panel<'a, Message: Clone + 'a>(
    state: SearchState<'a, Message>,
    footer: Option<(&'a str, Message)>,
) -> PanelParts<'a, Message> {
    let mut keys = Vec::new();
    let mut selections = Vec::new();
    let mut focus_targets = Vec::new();
    let highlight = Rc::new(Cell::new(None));
    let visible = !matches!(state, SearchState::Hidden);
    let body: Element<'a, Message> = match state {
        SearchState::Results(results) => {
            let mut rows = column![].width(Fill);

            for (index, result) in results.into_iter().enumerate() {
                keys.push(result.key.clone());
                selections.push(result.on_select.clone());
                rows = rows.push(result_row(
                    result,
                    Rc::clone(&highlight),
                    index,
                    &mut focus_targets,
                ));
            }

            container(rows).width(Fill).padding(spacing::MD).into()
        }
        SearchState::Loading => status_row("Searching…", false),
        SearchState::Empty => status_row("No results", false),
        SearchState::Error(error) => status_row(error, true),
        SearchState::Hidden => column![].into(),
    };

    let footer: Option<Element<'a, Message>> =
        footer.filter(|_| visible).map(|(label, message)| {
            let id = search_target("footer", "");
            focus_targets.push(id.clone());
            panel_footer(label, message).id(id).into()
        });
    let mut unique = HashSet::with_capacity(keys.len());
    assert!(
        keys.iter().all(|key| unique.insert(key.as_str())),
        "Search result keys must be unique"
    );

    PanelParts {
        panel: SearchPanel::new(scrollable(body).width(Fill), footer),
        visible,
        keys,
        selections,
        highlight,
        focus_targets,
    }
}

fn result_row<'a, Message: Clone + 'a>(
    result: SearchResult<'a, Message>,
    highlight: Rc<Cell<Option<usize>>>,
    index: usize,
    focus_targets: &mut Vec<Id>,
) -> Element<'a, Message> {
    let primary = search_target("result", &result.key);
    focus_targets.push(primary.clone());
    let mut content = row_content(result.title.into(), result.subtitle, result.icon);

    if let Some((label, icon, message)) = result.action {
        let action = search_target("action", &result.key);
        focus_targets.push(action.clone());
        content = content.push(
            Button::new(label)
                .trailing_icon(icon)
                .icon_rotation(if icon == Icon::Arrow {
                    std::f32::consts::PI
                } else {
                    0.0
                })
                .kind(ButtonKind::Surface)
                .id(action)
                .on_press(message),
        );
    }

    container(
        menu_item(content, Some(result.on_select), false, move || {
            highlight.get() == Some(index)
        })
        .id(primary),
    )
    .width(Fill)
    .into()
}

fn search_target(role: &str, key: &str) -> Id {
    format!("next-ui-search-{role}-{}:{key}", key.len()).into()
}

fn status_row<'a, Message: 'a>(label: &'a str, error: bool) -> Element<'a, Message> {
    container(text(label).label().style(move |theme: &Theme| text::Style {
        color: Some(if error {
            crate::theme::colors(theme).danger
        } else {
            crate::theme::colors(theme).muted
        }),
    }))
    .width(Fill)
    .padding(spacing::MD)
    .into()
}

struct SearchWidget<'a, Message> {
    input: Element<'a, Message>,
    panel: SearchPanel<'a, Message>,
    visible: bool,
    keys: Vec<String>,
    selections: Vec<Message>,
    highlight: Rc<Cell<Option<usize>>>,
    query: &'a str,
    on_submit: Option<Message>,
    focus_targets: Vec<Id>,
}

#[derive(Debug, Default)]
struct SearchLocal {
    input_focused: bool,
    panel_focus: PopupFocusState,
    dismissed: bool,
    restore_input_focus: bool,
    highlighted: Option<usize>,
    keys: Vec<String>,
    focus_targets: Vec<Id>,
    query: String,
}

impl SearchLocal {
    fn popup_open(&self, visible: bool) -> bool {
        visible
            && !self.dismissed
            && (self.input_focused || self.panel_focus != PopupFocusState::None)
    }
}

fn navigate(current: Option<usize>, count: usize, navigation: Navigation) -> Option<usize> {
    navigate_index(current, count, navigation, true)
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
            Tree::new(&self.panel as &dyn Widget<_, _, _>),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        let reset_panel = tree.state.downcast_ref::<SearchLocal>().keys != self.keys;
        tree.children[0].diff(&self.input);
        if reset_panel {
            tree.children[1] = Tree::new(&self.panel as &dyn Widget<_, _, _>);
        } else {
            tree.children[1].diff(&self.panel as &dyn Widget<_, _, _>);
        }
        let state = tree.state.downcast_mut::<SearchLocal>();
        let previous_highlight = state.highlighted;
        let focus_targets_changed = state.focus_targets != self.focus_targets;
        state.highlighted = reconcile_index(&state.keys, state.highlighted, &self.keys)
            .or((!self.keys.is_empty()).then_some(0));
        state.keys.clone_from(&self.keys);
        state.focus_targets.clone_from(&self.focus_targets);
        self.highlight.set(state.highlighted);

        if (reset_panel || focus_targets_changed) && state.panel_focus != PopupFocusState::None {
            state.panel_focus = PopupFocusState::None;
            state.restore_input_focus = true;
        }

        if state.query != self.query {
            state.query.clear();
            state.query.push_str(self.query);
            state.dismissed = false;
        }

        if !self.visible {
            state.panel_focus = PopupFocusState::None;
        } else if state.highlighted != previous_highlight
            && let Some(index) = state.highlighted
        {
            self.panel
                .reveal(search_target("result", &self.keys[index]));
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
        let restoring_input = state.restore_input_focus;

        if restoring_input {
            focus_first_descendant(
                self.input.as_widget_mut(),
                &mut tree.children[0],
                layout,
                renderer,
            );
            state.restore_input_focus = false;
            shell.request_redraw();
        }

        if state.dismissed
            && matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(_))
                    | Event::Touch(iced::touch::Event::FingerPressed { .. })
            )
            && event_cursor(event, cursor).is_over(layout.bounds())
        {
            state.dismissed = false;
        }

        let popup_open = state.popup_open(self.visible);

        if state.input_focused
            && popup_open
            && !self.focus_targets.is_empty()
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                modifiers,
                repeat: false,
                ..
            }) = event
            && !modifiers.shift()
        {
            state.panel_focus = PopupFocusState::Requested;
            unfocus_descendants(
                self.input.as_widget_mut(),
                &mut tree.children[0],
                layout,
                renderer,
            );
            state.input_focused = false;
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if state.input_focused
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key, repeat: false, ..
            }) = event
        {
            let result_count = self.selections.len();
            let navigation_key = matches!(
                key.as_ref(),
                keyboard::Key::Named(
                    key::Named::ArrowDown
                        | key::Named::ArrowUp
                        | key::Named::Home
                        | key::Named::End
                )
            );
            let handled = match key.as_ref() {
                keyboard::Key::Named(key::Named::ArrowDown) if popup_open && result_count > 0 => {
                    state.highlighted = navigate(state.highlighted, result_count, Navigation::Next);
                    true
                }
                keyboard::Key::Named(key::Named::ArrowUp) if popup_open && result_count > 0 => {
                    state.highlighted =
                        navigate(state.highlighted, result_count, Navigation::Previous);
                    true
                }
                keyboard::Key::Named(key::Named::Home) if popup_open && result_count > 0 => {
                    state.highlighted =
                        navigate(state.highlighted, result_count, Navigation::First);
                    true
                }
                keyboard::Key::Named(key::Named::End) if popup_open && result_count > 0 => {
                    state.highlighted = navigate(state.highlighted, result_count, Navigation::Last);
                    true
                }
                keyboard::Key::Named(key::Named::Enter) => {
                    if let Some(index) = state.highlighted.filter(|_| popup_open) {
                        state.dismissed = true;
                        shell.publish(self.selections[index].clone());
                        true
                    } else if let Some(message) = &self.on_submit {
                        state.dismissed = true;
                        shell.publish(message.clone());
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if handled {
                if navigation_key && let Some(index) = state.highlighted {
                    self.panel
                        .reveal(search_target("result", &self.keys[index]));
                }
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

        let input_focused = descendant_is_focused(
            self.input.as_widget_mut(),
            &mut tree.children[0],
            layout,
            renderer,
        );

        if input_focused && !state.input_focused && !restoring_input {
            state.dismissed = false;
            if let Some(index) = state.highlighted {
                self.panel
                    .reveal(search_target("result", &self.keys[index]));
            }
        }

        state.input_focused = input_focused;
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

        if !state.popup_open(self.visible) {
            return None;
        }

        let bounds = layout.bounds();

        let anchor = Rectangle::new(bounds.position() + translation, bounds.size());
        self.panel.width = anchor.width;

        let SearchLocal {
            panel_focus,
            dismissed,
            restore_input_focus,
            ..
        } = state;

        Some(overlay::Element::new(Box::new(PopupCore::new(
            anchor,
            *viewport,
            &mut self.panel,
            &mut children[1],
            0.0,
            PopupFocus {
                state: panel_focus,
                restore_anchor: restore_input_focus,
                targets: &self.focus_targets,
            },
            move |reason| {
                *dismissed = true;
                match reason {
                    DismissReason::OutsidePress | DismissReason::FocusLost => {
                        DismissOutcome::PassThrough
                    }
                    DismissReason::ContentMessage { keyboard: false } => DismissOutcome::Captured,
                    DismissReason::Escape | DismissReason::ContentMessage { keyboard: true } => {
                        DismissOutcome::CapturedAndRestoreFocus
                    }
                }
            },
        ))))
    }
}

struct SearchPanel<'a, Message> {
    children: Vec<Element<'a, Message>>,
    width: f32,
    reveal: Cell<Option<Id>>,
}

impl<'a, Message> SearchPanel<'a, Message> {
    fn new(body: impl Into<Element<'a, Message>>, footer: Option<Element<'a, Message>>) -> Self {
        Self {
            children: std::iter::once(body.into()).chain(footer).collect(),
            width: 0.0,
            reveal: Cell::new(None),
        }
    }

    fn reveal(&self, id: Id) {
        self.reveal.set(Some(id));
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for SearchPanel<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width);
        let width = limits.max().width;
        let max_height = limits.max().height;
        let (body, footer) = self.children.split_first_mut().expect("search panel body");
        let (body_tree, footer_trees) = tree
            .children
            .split_first_mut()
            .expect("search panel body tree");
        let child_limits = layout::Limits::new(Size::new(width, 0.0), Size::new(width, max_height));
        let footer = footer
            .first_mut()
            .zip(footer_trees.first_mut())
            .map(|(footer, tree)| footer.as_widget_mut().layout(tree, renderer, &child_limits));
        let footer_height = footer.as_ref().map_or(0.0, |node| node.size().height);
        let body = body.as_widget_mut().layout(
            body_tree,
            renderer,
            &child_limits.max_height((max_height - footer_height).max(0.0)),
        );
        let body_height = body.size().height;
        let mut children = vec![body];

        if let Some(footer) = footer {
            children.push(footer.move_to(iced::Point::new(0.0, body_height)));
        }

        layout::Node::with_children(
            Size::new(width, (body_height + footer_height).min(max_height)),
            children,
        )
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
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        if let Some(id) = self.reveal.take()
            && reveal(self, tree, layout, renderer, &id)
        {
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
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let theme = SurfaceRole::Overlay.draw_background(renderer, theme, layout.bounds());
        let style = renderer::Style {
            text_color: crate::theme::colors(&theme).text,
        };

        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, &theme, &style, layout, cursor, viewport);
        }
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
        .map(|content| scoped_overlay(SurfaceRole::Overlay, content))
    }
}

fn search_style(theme: &Theme) -> container::Style {
    let colors = crate::theme::colors(theme);
    crate::theme::surface(iced::theme::palette::Pair {
        color: colors.border,
        text: colors.text,
    })
}

fn input_style(theme: &Theme) -> text_input::Style {
    let colors = crate::theme::colors(theme);

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: colors.muted,
        placeholder: colors.muted,
        value: colors.text,
        selection: colors.accent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_highlight_across_result_changes() {
        let old = ["a", "b", "c"].map(String::from);
        let removed = ["a", "c"].map(String::from);
        let empty: [String; 0] = [];

        assert_eq!(
            reconcile_index(&old, Some(1), &["c", "a", "b"].map(String::from),),
            Some(2)
        );
        assert_eq!(
            reconcile_index(&old, Some(1), &removed).or((!removed.is_empty()).then_some(0)),
            Some(0)
        );
        assert_eq!(
            reconcile_index(&old, Some(1), &empty).or((!empty.is_empty()).then_some(0)),
            None
        );
    }

    #[test]
    fn reordering_results_restores_focus_to_the_input() {
        fn search(results: [&'static str; 2]) -> Element<'static, ()> {
            Search::new("Search", "query", |_| ())
                .state(SearchState::Results(
                    results.map(|key| SearchResult::new(key, key, ())).into(),
                ))
                .into()
        }

        let first = search(["first", "second"]);
        let mut tree = Tree::new(&first);
        let state = tree.state.downcast_mut::<SearchLocal>();
        state.keys = ["first", "second"].map(String::from).into();
        state.highlighted = Some(0);
        state.panel_focus = PopupFocusState::Target(0);

        tree.diff(search(["second", "first"]));

        let state = tree.state.downcast_ref::<SearchLocal>();
        assert_eq!(state.highlighted, Some(1));
        assert_eq!(state.panel_focus, PopupFocusState::None);
        assert!(state.restore_input_focus);
    }

    #[test]
    #[should_panic(expected = "Search result keys must be unique")]
    fn duplicate_result_keys_are_rejected() {
        let _: Element<'_, ()> = Search::new("Search", "", |_| ())
            .state(SearchState::Results(vec![
                SearchResult::new("same", "First", ()),
                SearchResult::new("same", "Second", ()),
            ]))
            .into();
    }

    #[test]
    fn input_keeps_the_original_border_and_selection() {
        let theme = crate::theme::dark();
        let colors = crate::theme::colors(&theme);
        let style = input_style(&theme);

        assert_eq!(style.border, Border::default());
        assert_eq!(style.selection, colors.accent);
    }
}
