use iced::{
    Alignment, Background, Border, ContentFit, Element, Fill, Theme,
    widget::{Column, Space, button, column, container, row, svg, text},
};

pub struct SelectorRow<'a, T, Message> {
    title: &'a str,
    placeholder: &'a str,
    options: &'a [T],
    selected: Option<&'a T>,
    icon: Option<svg::Handle>,
    expanded: bool,
    on_selected: Box<dyn Fn(T) -> Message + 'a>,
    on_toggle: Message,
}

impl<'a, T, Message> SelectorRow<'a, T, Message> {
    pub fn new(
        options: &'a [T],
        on_selected: impl Fn(T) -> Message + 'a,
        on_toggle: Message,
    ) -> Self {
        Self {
            title: "",
            placeholder: "",
            options,
            selected: None,
            icon: None,
            expanded: false,
            on_selected: Box::new(on_selected),
            on_toggle,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn icon(mut self, icon: impl Into<svg::Handle>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn selected(mut self, selected: Option<&'a T>) -> Self {
        self.selected = selected;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

impl<'a, T, Message> From<SelectorRow<'a, T, Message>> for Element<'a, Message>
where
    T: Clone + PartialEq + ToString + 'a,
    Message: Clone + 'a,
{
    fn from(selector: SelectorRow<'a, T, Message>) -> Self {
        let expanded = selector.expanded;
        let value = selector
            .selected
            .map(ToString::to_string)
            .unwrap_or_else(|| selector.placeholder.to_owned());

        let mut value_row = row![].spacing(12).align_y(Alignment::Center);

        if let Some(icon) = selector.icon {
            value_row = value_row.push(icon_view(icon));
        }

        value_row = value_row.push(text(value).size(16).style(muted_text));

        let header = button(
            row![
                column![text(selector.title).size(18), value_row,]
                    .width(Fill)
                    .spacing(8),
                svg(crate::icons::get("down_caret"))
                    .width(20)
                    .height(20)
                    .content_fit(ContentFit::Contain)
                    .rotation(if expanded { std::f32::consts::PI } else { 0.0 }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([18, 24])
        .on_press(selector.on_toggle)
        .style(header_style);

        let mut content = column![header].width(Fill);

        if expanded {
            let selected = selector.selected;
            let on_selected = selector.on_selected;
            let rows = selector.options.iter().map(move |option| {
                let is_selected = selected == Some(option);

                button(text(option.to_string()).size(18))
                    .width(Fill)
                    .padding([10, 9])
                    .on_press(on_selected(option.clone()))
                    .style(move |theme, status| option_style(theme, status, is_selected))
                    .into()
            });

            content = content
                .push(
                    container(Space::new())
                        .width(Fill)
                        .height(1)
                        .style(divider_style),
                )
                .push(
                    container(Column::with_children(rows).width(Fill))
                        .width(Fill)
                        .padding([16, 10]),
                );
        }

        container(content)
            .width(Fill)
            .clip(true)
            .style(move |theme| surface_style(theme, expanded))
            .into()
    }
}

fn icon_view<'a, Message: 'a>(handle: svg::Handle) -> Element<'a, Message> {
    svg(handle)
        .width(16)
        .height(16)
        .content_fit(ContentFit::Contain)
        .into()
}

fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().secondary.base.text),
    }
}

fn surface_style(theme: &Theme, expanded: bool) -> container::Style {
    let background = if expanded {
        theme.extended_palette().background.neutral.color
    } else {
        theme.extended_palette().background.weak.color
    };

    container::Style::default()
        .background(background)
        .border(Border::default().rounded(8))
}

fn header_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(
            theme.extended_palette().background.neutral.color,
        )),
        button::Status::Pressed => Some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        _ => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

fn divider_style(theme: &Theme) -> container::Style {
    container::Style::default().background(theme.extended_palette().background.stronger.color)
}

fn option_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let highlighted =
        selected || matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: highlighted.then_some(Background::Color(
            theme.extended_palette().background.stronger.color,
        )),
        text_color: if highlighted {
            theme.palette().text
        } else {
            theme.extended_palette().secondary.base.text
        },
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::{Background, widget::button};

    use crate::theme;

    use super::{divider_style, option_style, surface_style};

    #[test]
    fn expanded_and_selected_states_match_the_mockup() {
        let theme = theme::theme();

        assert_eq!(
            surface_style(&theme, true).background,
            Some(Background::Color(theme::BORDER))
        );
        assert_eq!(
            option_style(&theme, button::Status::Active, true).background,
            Some(Background::Color(theme::SURFACE_SELECTED))
        );
        assert_eq!(
            divider_style(&theme).background,
            Some(Background::Color(theme::SURFACE_SELECTED))
        );
    }
}
