use iced::{
    Alignment, ContentFit, Element, Fill,
    widget::{Space, button, column, container, row, stack, svg, text},
};

use crate::icons;

use super::style;

pub struct ActionRow<'a, Message> {
    title: &'a str,
    description: &'a str,
    icon: Option<svg::Handle>,
    on_press: Option<Message>,
    progress: Option<u8>,
}

impl<Message> ActionRow<'_, Message> {
    pub fn new() -> Self {
        Self {
            title: "",
            description: "",
            icon: None,
            on_press: None,
            progress: None,
        }
    }
}

impl<'a, Message> ActionRow<'a, Message> {
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub fn icon(mut self, icon: impl Into<svg::Handle>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress.min(100));
        self
    }
}

impl<Message> Default for ActionRow<'_, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<ActionRow<'a, Message>> for Element<'a, Message> {
    fn from(action: ActionRow<'a, Message>) -> Self {
        let mut description = row![].spacing(16).align_y(Alignment::Center);

        if let Some(icon) = action.icon {
            description = description.push(
                svg(icon)
                    .width(24)
                    .height(24)
                    .content_fit(ContentFit::Contain),
            );
        }

        description = description.push(text(action.description).size(16).style(style::muted_text));

        let labels =
            column![text(action.title).size(18).style(text::base), description,].spacing(4);

        let trailing: Element<'a, Message> = match action.progress {
            Some(progress) => progress_indicator(progress),
            None => svg(icons::get("arrow"))
                .width(24)
                .height(24)
                .content_fit(ContentFit::Contain)
                .rotation(std::f32::consts::PI)
                .into(),
        };

        let row =
            button(row![labels, Space::new().width(Fill), trailing].align_y(Alignment::Center))
                .padding([18, 24])
                .width(Fill)
                .style(style::action);

        match (action.on_press, action.progress) {
            (Some(on_press), None) => row.on_press(on_press).into(),
            _ => row.into(),
        }
    }
}

fn progress_indicator<'a, Message: 'a>(progress: u8) -> Element<'a, Message> {
    let circumference = 2.0 * std::f32::consts::PI * 36.0;
    let filled = circumference * f32::from(progress) / 100.0;
    let ring = format!(
        r##"<svg viewBox="0 0 80 80" xmlns="http://www.w3.org/2000/svg">
<circle cx="40" cy="40" r="36" fill="none" stroke="#594E52" stroke-width="4"/>
<circle cx="40" cy="40" r="36" fill="none" stroke="#A6939A" stroke-width="4"
stroke-dasharray="{filled} {circumference}" transform="rotate(-90 40 40)"/>
</svg>"##
    );

    stack![
        svg(svg::Handle::from_memory(ring.into_bytes()))
            .width(40)
            .height(40),
        container(
            column![
                text(progress).size(14).style(text::base),
                text("%").size(14).style(text::base),
            ]
            .align_x(Alignment::Center),
        )
        .center(40),
    ]
    .width(40)
    .height(40)
    .into()
}

#[cfg(test)]
mod tests {
    use super::ActionRow;

    #[test]
    fn progress_is_clamped() {
        let row = ActionRow::<()>::new().progress(150);

        assert_eq!(row.progress, Some(100));
    }
}
