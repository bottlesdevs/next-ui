use iced::{
    Background, Border, Center, Color, Element, Fill, Pixels, Theme,
    widget::{Column, Space, button, column, container, row, text, text_input},
};

use crate::icons::Icon;

use super::{button::Button, text::TextExt as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Install,
    Run,
}

impl Action {
    const fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Run => "Run",
        }
    }
}

pub struct SearchResultRow<'a, Message> {
    title: &'a str,
    action: Option<Action>,
    on_press: Message,
}

impl<'a, Message> SearchResultRow<'a, Message> {
    pub fn new(title: &'a str, on_press: Message) -> Self {
        Self {
            title,
            action: None,
            on_press,
        }
    }

    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SearchResultRow<'a, Message>> for Element<'a, Message> {
    fn from(result: SearchResultRow<'a, Message>) -> Self {
        let mut content = row![text(result.title).subtitle()]
            .spacing(16)
            .align_y(Center);

        if let Some(result_action) = result.action {
            content = content
                .push(Space::new().width(Fill))
                .push(action(result_action));
        }

        button(content)
            .padding([14, 20])
            .width(Fill)
            .style(result_style)
            .on_press(result.on_press)
            .into()
    }
}

pub struct Search<'a, Message> {
    placeholder: &'a str,
    query: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    results: Option<Vec<SearchResultRow<'a, Message>>>,
    footer: Option<(&'a str, Message)>,
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
            results: None,
            footer: None,
            padding_x: 20.0,
            padding_y: 16.0,
        }
    }

    pub fn results(
        mut self,
        results: impl IntoIterator<Item = SearchResultRow<'a, Message>>,
    ) -> Self {
        self.results = Some(results.into_iter().collect());
        self
    }

    pub fn footer(mut self, label: &'a str, on_press: Message) -> Self {
        self.footer = Some((label, on_press));
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
        let Search {
            placeholder,
            query,
            on_input,
            results,
            footer,
            padding_x,
            padding_y,
        } = search;
        let expanded = should_expand(query, results.is_some());
        let mut content = column![search_input(
            placeholder,
            query,
            on_input,
            expanded,
            padding_x,
            padding_y
        )]
        .width(Fill);

        if expanded {
            let results = results
                .expect("results exist when the search is expanded")
                .into_iter()
                .map(Element::from);

            content = content
                .push(container(Column::with_children(results).width(Fill)).padding([0, 20]));

            if let Some((label, on_press)) = footer {
                content = content.push(
                    button(
                        row![text(label), Icon::Arrow.rotated(std::f32::consts::PI),]
                            .spacing(14)
                            .align_y(Center),
                    )
                    .padding([22, 40])
                    .width(Fill)
                    .style(footer_style)
                    .on_press(on_press),
                );
            }
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(move |theme| {
                if expanded {
                    panel_style(theme)
                } else {
                    container::Style::default()
                }
            })
            .into()
    }
}

fn should_expand(query: &str, expandable: bool) -> bool {
    expandable && !query.trim().is_empty()
}

fn search_input<'a, Message: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    embedded: bool,
    padding_x: f32,
    padding_y: f32,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .width(Fill)
        .padding(0)
        .size(18)
        .style(input_style);

    container(row![Icon::Search.view(), input].spacing(12).align_y(Center))
        .width(Fill)
        .padding([padding_y, padding_x])
        .style(move |theme| search_style(theme, embedded))
        .into()
}

fn search_style(theme: &Theme, embedded: bool) -> container::Style {
    let colors = theme.extended_palette();

    container::Style {
        background: Some(Background::Color(if embedded {
            Color::TRANSPARENT
        } else {
            colors.background.neutral.color
        })),
        border: if embedded {
            Border::default()
        } else {
            Border::default().rounded(8)
        },
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

fn action<'a, Message: Clone + 'a>(action: Action) -> Element<'a, Message> {
    match action {
        Action::Install => button(
            row![
                text(action.label()).label(),
                Icon::Arrow.rotated(std::f32::consts::PI),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([10, 16])
        .style(install_action_style)
        .into(),
        Action::Run => Button::new(action.label())
            .icon(Icon::Play)
            .circular()
            .surface()
            .into(),
    }
}

fn install_action_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let colors = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strong,
        _ => palette.background.weak,
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: colors.text,
        border: Border::default().rounded(999),
        ..button::Style::default()
    }
}

fn panel_style(theme: &Theme) -> container::Style {
    let colors = theme.extended_palette().background.neutral;

    container::Style {
        text_color: Some(colors.text),
        background: Some(Background::Color(colors.color)),
        border: Border::default().rounded(12),
        ..container::Style::default()
    }
}

fn result_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let highlighted = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: highlighted.then_some(Background::Color(palette.background.stronger.color)),
        text_color: if highlighted {
            palette.background.stronger.text
        } else {
            palette.secondary.weak.text
        },
        border: Border::default().rounded(10),
        ..button::Style::default()
    }
}

fn footer_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let colors = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strongest,
        _ => palette.background.stronger,
    };

    button::Style {
        background: Some(Background::Color(colors.color)),
        text_color: palette.secondary.weak.text,
        border: Border::default().rounded(iced::border::bottom(12)),
        ..button::Style::default()
    }
}
