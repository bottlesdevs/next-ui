use std::{cell::Cell, rc::Rc};

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
    anchored_panel::{
        AnchoredPanel, PanelContent, footer as panel_footer, row_content,
        row_style as panel_row_style,
    },
    button::{Button, ButtonKind},
    control::{Control, descendant_is_focused},
    reconcile_index, spacing,
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
        let (panel, visible, keys, selections, highlight) = panel(search.state, search.footer);

        Element::new(SearchWidget {
            input: input.into(),
            panel,
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
    PanelContent<'a, Message>,
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

    let footer = footer
        .filter(|_| visible)
        .map(|(label, message)| panel_footer(label, message));

    (
        PanelContent::new(scrollable(body).width(Fill), footer),
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
    let mut content = row_content(result.title, result.subtitle, result.icon);

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

    Control::new(content)
        .width(Fill)
        .padding([spacing::XS, spacing::MD])
        .on_press(result.on_select)
        .style(move |theme, mut state| {
            state.keyboard_highlighted = highlight.get() == Some(index);
            panel_row_style(theme, state)
        })
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
    panel: PanelContent<'a, Message>,
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
        vec![Tree::new(&self.input), self.panel.tree()]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.input);
        self.panel.diff(&mut tree.children[1]);
        let state = tree.state.downcast_mut::<SearchLocal>();
        state.highlighted = reconcile_index(&state.keys, state.highlighted, &self.keys)
            .or((!self.keys.is_empty()).then_some(0));
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

        let focused =
            descendant_is_focused(&mut self.input, &mut tree.children[0], layout, renderer);

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

        let bounds = layout.bounds();

        Some(overlay::Element::new(Box::new(AnchoredPanel::search(
            bounds.position() + translation,
            bounds.height,
            bounds.width,
            *viewport,
            &mut self.panel,
            &mut children[1],
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::reconcile_index;

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
}

fn search_style(theme: &Theme) -> container::Style {
    crate::theme::surface(theme.extended_palette().background.neutral)
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
