use iced::{
    Alignment, Background, Border, Color, Element, Fill,
    widget::{Space, button, column, container, row, text, text_input, toggler},
};

use super::{popover::Popover, style};

pub struct Field<'a, Message> {
    content: Element<'a, Message>,
}

impl<'a, Message> Field<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<'a, Message: 'a> From<Field<'a, Message>> for Element<'a, Message> {
    fn from(field: Field<'a, Message>) -> Self {
        container(field.content)
            .width(Fill)
            .padding([18, 20])
            .style(style::surface)
            .into()
    }
}

pub struct Editable<'a, Message> {
    label: &'a str,
    value: &'a str,
    on_edit: Message,
}

impl<'a, Message> Editable<'a, Message> {
    pub fn new(label: &'a str, value: &'a str, on_edit: Message) -> Self {
        Self {
            label,
            value,
            on_edit,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Editable<'a, Message>> for Element<'a, Message> {
    fn from(field: Editable<'a, Message>) -> Self {
        button(
            row![
                description(field.label, field.value),
                text("✎").size(22).style(style::muted_text),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .width(Fill)
        .padding([18, 20])
        .on_press(field.on_edit)
        .style(style::action)
        .into()
    }
}

pub struct Input<'a, Message> {
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
}

impl<'a, Message> Input<'a, Message> {
    pub fn new(
        label: &'a str,
        placeholder: &'a str,
        value: &'a str,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        Self {
            label,
            placeholder,
            value,
            on_input: Box::new(on_input),
        }
    }
}

impl<'a, Message: Clone + 'a> From<Input<'a, Message>> for Element<'a, Message> {
    fn from(field: Input<'a, Message>) -> Self {
        Field::new(
            column![
                text(field.label).size(18),
                text_input(field.placeholder, field.value)
                    .on_input(field.on_input)
                    .padding(0)
                    .size(16)
                    .style(input_style),
            ]
            .spacing(8),
        )
        .into()
    }
}

pub struct Disabled<'a> {
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
}

impl<'a> Disabled<'a> {
    pub fn new(label: &'a str, placeholder: &'a str, value: &'a str) -> Self {
        Self {
            label,
            placeholder,
            value,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Disabled<'a>> for Element<'a, Message> {
    fn from(field: Disabled<'a>) -> Self {
        let content: Element<'a, Message> = column![
            text(field.label).size(18).style(style::muted_text),
            text_input(field.placeholder, field.value)
                .padding(0)
                .size(16)
                .style(input_style),
        ]
        .spacing(8)
        .into();

        Field::new(content).into()
    }
}

pub struct Selector<'a, T, Message> {
    label: &'a str,
    placeholder: &'a str,
    options: &'a [T],
    selected: Option<&'a T>,
    expanded: bool,
    on_selected: Box<dyn Fn(T) -> Message + 'a>,
    on_toggle: Message,
}

impl<'a, T, Message> Selector<'a, T, Message> {
    pub fn new(
        label: &'a str,
        placeholder: &'a str,
        options: &'a [T],
        selected: Option<&'a T>,
        on_selected: impl Fn(T) -> Message + 'a,
        on_toggle: Message,
    ) -> Self {
        Self {
            label,
            placeholder,
            options,
            selected,
            expanded: false,
            on_selected: Box::new(on_selected),
            on_toggle,
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

impl<'a, T, Message> From<Selector<'a, T, Message>> for Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    Message: Clone + 'a,
{
    fn from(field: Selector<'a, T, Message>) -> Self {
        let selected_index = field
            .selected
            .and_then(|selected| field.options.iter().position(|option| option == selected));
        let value = field
            .selected
            .map(ToString::to_string)
            .unwrap_or_else(|| field.placeholder.to_owned());
        let header = button(
            column![
                text(field.label).size(18),
                row![
                    text(value).size(16).style(style::muted_text),
                    Space::new().width(Fill),
                    text(if field.expanded { "⌃" } else { "⌄" })
                        .size(26)
                        .style(style::muted_text),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding([18, 20])
        .on_press(field.on_toggle)
        .style(selector_header_style);

        let mut content = column![header].width(Fill);

        if field.expanded {
            let options = field.options;
            let on_selected = field.on_selected;

            content = content
                .push(
                    container(Space::new())
                        .height(1)
                        .width(Fill)
                        .style(selector_divider_style),
                )
                .push(
                    Popover::new(options, move |index| on_selected(options[index].clone()))
                        .selected(selected_index)
                        .embedded(),
                );
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(style::surface)
            .into()
    }
}

pub struct Action<'a, Message> {
    title: &'a str,
    description_text: &'a str,
    on_press: Message,
}

impl<'a, Message> Action<'a, Message> {
    pub fn new(title: &'a str, description_text: &'a str, on_press: Message) -> Self {
        Self {
            title,
            description_text,
            on_press,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Action<'a, Message>> for Element<'a, Message> {
    fn from(field: Action<'a, Message>) -> Self {
        button(
            row![
                description(field.title, field.description_text),
                text("→").size(26).style(style::muted_text),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .width(Fill)
        .padding([18, 20])
        .on_press(field.on_press)
        .style(style::action)
        .into()
    }
}

pub struct Collapsible<'a, Message> {
    title: &'a str,
    description_text: &'a str,
    expanded: bool,
    on_press: Message,
}

impl<'a, Message> Collapsible<'a, Message> {
    pub fn new(title: &'a str, description_text: &'a str, on_press: Message) -> Self {
        Self {
            title,
            description_text,
            expanded: false,
            on_press,
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Collapsible<'a, Message>> for Element<'a, Message> {
    fn from(field: Collapsible<'a, Message>) -> Self {
        button(
            row![
                description(field.title, field.description_text),
                text(if field.expanded { "⌃" } else { "⌄" })
                    .size(26)
                    .style(style::muted_text),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .width(Fill)
        .padding([18, 20])
        .on_press(field.on_press)
        .style(style::action)
        .into()
    }
}

pub struct Toggle<'a, Message> {
    title: &'a str,
    description_text: &'a str,
    value: bool,
    on_toggle: Box<dyn Fn(bool) -> Message + 'a>,
}

impl<'a, Message> Toggle<'a, Message> {
    pub fn new(
        title: &'a str,
        description_text: &'a str,
        value: bool,
        on_toggle: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        Self {
            title,
            description_text,
            value,
            on_toggle: Box::new(on_toggle),
        }
    }
}

impl<'a, Message: Clone + 'a> From<Toggle<'a, Message>> for Element<'a, Message> {
    fn from(field: Toggle<'a, Message>) -> Self {
        Field::new(
            row![
                description(field.title, field.description_text),
                toggler(field.value).on_toggle(field.on_toggle),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .into()
    }
}

pub struct Value<'a, Message> {
    title: &'a str,
    current: &'a str,
    previous: Message,
    next: Message,
}

impl<'a, Message> Value<'a, Message> {
    pub fn new(title: &'a str, current: &'a str, previous: Message, next: Message) -> Self {
        Self {
            title,
            current,
            previous,
            next,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Value<'a, Message>> for Element<'a, Message> {
    fn from(field: Value<'a, Message>) -> Self {
        Field::new(
            row![
                button("←")
                    .on_press(field.previous)
                    .style(style::tab)
                    .padding(0),
                column![
                    text(field.title).size(18),
                    text(field.current).size(16).style(style::muted_text),
                ]
                .width(Fill)
                .align_x(Alignment::Center)
                .spacing(4),
                button("→")
                    .on_press(field.next)
                    .style(style::tab)
                    .padding(0),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .into()
    }
}

pub struct Path<'a, Message> {
    title: &'a str,
    path: &'a str,
    on_choose: Message,
}

impl<'a, Message> Path<'a, Message> {
    pub fn new(title: &'a str, path: &'a str, on_choose: Message) -> Self {
        Self {
            title,
            path,
            on_choose,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Path<'a, Message>> for Element<'a, Message> {
    fn from(field: Path<'a, Message>) -> Self {
        Field::new(
            row![
                description(field.title, field.path),
                button("▰")
                    .on_press(field.on_choose)
                    .style(style::tab)
                    .padding(0),
            ]
            .align_y(Alignment::Center)
            .spacing(16),
        )
        .into()
    }
}

fn description<'a, Message: 'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![
        text(title).size(18),
        text(description).size(16).style(style::muted_text),
    ]
    .width(Fill)
    .spacing(4)
    .into()
}

fn input_style(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let colors = theme.extended_palette();

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: colors.secondary.weak.text,
        placeholder: colors.secondary.weak.text,
        value: match status {
            text_input::Status::Disabled => colors.secondary.weak.text,
            _ => theme.palette().text,
        },
        selection: colors.primary.weak.color,
    }
}

fn selector_header_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut appearance = style::tab(theme, status);

    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        appearance.background = Some(Background::Color(
            theme.extended_palette().secondary.strong.color,
        ));
    }

    appearance
}

fn selector_divider_style(theme: &iced::Theme) -> container::Style {
    container::Style::default().background(theme.extended_palette().background.neutral.color)
}
