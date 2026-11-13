use iced::{
    Alignment, Background, Border, Color, Element, Fill, Theme,
    advanced::widget::Id,
    widget::{button, column, container, row, scrollable, text, text_input},
};

use crate::icons::Icon;

use super::{
    focusable_action::FocusableAction,
    list_row::{self, ListRow, Menu},
    popup::{AnchorMessage, Popup, PopupMessage},
    spacing,
    text::TextExt as _,
};

pub enum SearchState<'a, Message> {
    Hidden,
    Loading,
    /// Rows to pick from, each pressed as a whole and possibly holding a
    /// trailing control.
    Results(Vec<ListRow<'a, Message>>),
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
        let (panel, visible, focusable) = panel(search.state, search.footer);

        Popup::new(
            input.map(AnchorMessage::Emit),
            panel.map(PopupMessage::Close),
        )
        .while_focused(visible, search.query)
        .focusable(focusable)
        .match_anchor_width()
        .into()
    }
}

fn panel<'a, Message: Clone + 'a>(
    state: SearchState<'a, Message>,
    footer: Option<(&'a str, Message)>,
) -> (Element<'a, Message>, bool, bool) {
    let visible = !matches!(state, SearchState::Hidden);
    let mut focusable = matches!(&state, SearchState::Results(results) if !results.is_empty());
    let body: Element<'a, Message> = match state {
        SearchState::Results(results) => container(
            column(
                results
                    .into_iter()
                    .map(|result| result.menu(Menu::Floating).into()),
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
        focusable = true;
        content = content.push(footer_row(label, message));
    }

    (
        container(content).style(crate::theme::overlay).into(),
        visible,
        focusable,
    )
}

/// A row across the panel's foot, e.g. a fallback for when no result fits.
fn footer_row<'a, Message: Clone + 'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    FocusableAction::new(
        row![
            text(label),
            Icon::Arrow.view().rotation(std::f32::consts::PI)
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(spacing::MD)
    .on_press_maybe(Some(message))
    .style(|theme, state| button::Style {
        background: Some(Background::Color(crate::theme::colors(theme).hover)),
        border: Border::default().rounded(iced::border::bottom(6)),
        ..list_row::menu_style(theme, state, Menu::Floating)
    })
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
