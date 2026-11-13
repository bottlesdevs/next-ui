use iced::{
    Alignment, Background, Border, Color, Element, Fill, Theme,
    advanced::widget::Id,
    widget::{column, container, row, scrollable, text, text_input},
};

use crate::icons::Icon;

use super::{
    button::{Button, ButtonKind},
    menu::{footer as panel_footer, overlay_item as menu_item, row_content},
    popup::{AnchorMessage, Popup, PopupMessage},
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
        let input: Element<'a, Message> = input.into();
        let (panel, visible, focus_targets) = panel(search.state, search.footer);

        Popup::new(
            input.map(AnchorMessage::Emit),
            panel.map(PopupMessage::Close),
        )
        .while_focused(visible, search.query)
        .focus_targets(focus_targets)
        .match_anchor_width()
        .into()
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
