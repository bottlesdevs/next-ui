//! Placeholder boundary for the not-yet-implemented Next experience.

use iced::{
    Element,
    widget::{column, text},
};

use crate::widget::{Button, ButtonKind};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    UseClassic,
}

pub struct State;

impl State {
    pub fn new() -> Self {
        Self
    }

    pub fn view(&self) -> Element<'_, Action> {
        column![
            text("Next experience is not available yet").size(32),
            text("Choose Classic to use Bottles today."),
            Button::new("Use Classic")
                .kind(ButtonKind::Primary)
                .on_press(Action::UseClassic),
        ]
        .spacing(12)
        .into()
    }
}
