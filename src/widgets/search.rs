use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Rectangle, Size, Theme,
    Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Id, Operation, Tree, tree},
    },
    keyboard::{self, key},
    widget::{column, container, row, scrollable, text, text_input},
};

use crate::icons::Icon;

use super::{
    anchored_overlay::{DismissOutcome, DismissReason, PopupCore, PopupFocus, PopupFocusState},
    button::{Button, ButtonKind},
    focus::{descendant_is_focused, event_cursor, focus_first_descendant, unfocus_descendants},
    menu::{footer as panel_footer, item as menu_item, row_content},
    spacing,
    text::TextExt as _,
};

pub struct SearchResult<'a, Message> {
    title: &'a str,
    subtitle: Option<&'a str>,
    icon: Option<Icon>,
    on_select: Message,
    action: Option<(&'a str, Icon, Message)>,
}

impl<'a, Message> SearchResult<'a, Message> {
    pub fn new(title: &'a str, on_select: Message) -> Self {
        Self {
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
            .on_submit_maybe(search.on_submit)
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
        let (panel, visible, focus_targets) = panel(search.state, search.footer);

        Element::new(SearchWidget {
            input: input.into(),
            panel,
            visible,
            focus_targets,
            query: search.query,
        })
    }
}

fn panel<'a, Message: Clone + 'a>(
    state: SearchState<'a, Message>,
    footer: Option<(&'a str, Message)>,
) -> (Element<'a, Message>, bool, Vec<Id>) {
    let mut focus_targets = Vec::new();
    let visible = !matches!(state, SearchState::Hidden);
    let body: Element<'a, Message> = match state {
        SearchState::Results(results) => container(
            column(
                results
                    .into_iter()
                    .map(|result| result_row(result, &mut focus_targets)),
            )
            .width(Fill),
        )
        .width(Fill)
        .padding(spacing::MD)
        .into(),
        SearchState::Loading => status_row("Searching…", false),
        SearchState::Empty => status_row("No results", false),
        SearchState::Error(error) => status_row(error, true),
        SearchState::Hidden => column![].into(),
    };
    let mut content = column![scrollable(body).width(Fill)].width(Fill);

    if let Some((label, message)) = footer.filter(|_| visible) {
        let id = Id::unique();
        focus_targets.push(id.clone());
        content = content.push(panel_footer(label, message).id(id));
    }

    (
        container(content).style(crate::theme::overlay).into(),
        visible,
        focus_targets,
    )
}

fn result_row<'a, Message: Clone + 'a>(
    result: SearchResult<'a, Message>,
    focus_targets: &mut Vec<Id>,
) -> Element<'a, Message> {
    let primary = Id::unique();
    focus_targets.push(primary.clone());
    let mut content = row_content(result.title.into(), result.subtitle, result.icon);

    if let Some((label, icon, message)) = result.action {
        let id = Id::unique();
        focus_targets.push(id.clone());
        content = content.push(
            Button::new(label)
                .trailing_icon(icon)
                .icon_rotation(if icon == Icon::Arrow {
                    std::f32::consts::PI
                } else {
                    0.0
                })
                .kind(ButtonKind::Surface)
                .id(id)
                .on_press(message),
        );
    }

    container(menu_item(content, Some(result.on_select), false).id(primary))
        .width(Fill)
        .into()
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
    panel: Element<'a, Message>,
    visible: bool,
    focus_targets: Vec<Id>,
    query: &'a str,
}

#[derive(Debug, Default)]
struct SearchLocal {
    input_focused: bool,
    panel_focus: PopupFocusState,
    dismissed: bool,
    restore_input_focus: bool,
    query: String,
}

impl SearchLocal {
    fn popup_open(&self, visible: bool) -> bool {
        visible
            && !self.dismissed
            && (self.input_focused || self.panel_focus != PopupFocusState::None)
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for SearchWidget<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SearchLocal>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SearchLocal {
            query: self.query.to_owned(),
            ..SearchLocal::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.input), Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.input, &self.panel]);
        let state = tree.state.downcast_mut::<SearchLocal>();

        if state.query != self.query {
            state.query.clear();
            state.query.push_str(self.query);
            state.dismissed = false;
        }

        if !self.visible {
            state.panel_focus = PopupFocusState::None;
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

        if state.restore_input_focus {
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

        if state.input_focused
            && state.popup_open(self.visible)
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

        if input_focused && !state.input_focused {
            state.dismissed = false;
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
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, iced::Renderer>> {
        let (state, children) = (&mut tree.state, &mut tree.children);
        let state = state.downcast_mut::<SearchLocal>();

        if !state.popup_open(self.visible) {
            return self.input.as_widget_mut().overlay(
                &mut children[0],
                layout,
                renderer,
                viewport,
                translation,
            );
        }

        let anchor = Rectangle::new(
            layout.bounds().position() + translation,
            layout.bounds().size(),
        );
        let SearchLocal {
            panel_focus,
            dismissed,
            restore_input_focus,
            ..
        } = state;

        Some(overlay::Element::new(Box::new(
            PopupCore::new(
                anchor,
                *viewport,
                self.panel.as_widget_mut(),
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
                        DismissReason::ContentMessage { keyboard: false } => {
                            DismissOutcome::Captured
                        }
                        DismissReason::Escape
                        | DismissReason::ContentMessage { keyboard: true } => {
                            DismissOutcome::CapturedAndRestoreFocus
                        }
                    }
                },
            )
            .match_anchor_width(),
        )))
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
    fn input_keeps_the_original_border_and_selection() {
        let theme = crate::theme::dark();
        let colors = crate::theme::colors(&theme);
        let style = input_style(&theme);

        assert_eq!(style.border, Border::default());
        assert_eq!(style.selection, colors.accent);
    }
}
