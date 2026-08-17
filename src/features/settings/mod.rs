//! Bottle "Settings" detail tab: wrapper toggles (Gamescope/MangoHud) and
//! bottle/runner info. Stateless — everything it reads comes from the
//! currently-selected bottle, passed in via [`Context`] from the shell.

use std::sync::Arc;

use bottles_core::{Bottle, BottleState, MangoHudConfig};
use iced::{
    Element,
    widget::{column, container, svg, text},
};

use crate::{
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        list_row::ListRow,
        picker_row::PickerRow,
        row_group::RowGroup,
        switcher_row::SwitcherRow,
        text::TextExt as _,
    },
    icons::Icon,
};

pub struct Context<'a> {
    pub bottle: Option<Bottle>,
    pub bottle_state: Option<&'a Arc<BottleState>>,
}

#[derive(Clone)]
pub enum Message {
    ToggleGamescope(bool),
    ToggleMangoHud(bool),
    WrapperUpdated(Result<(), String>),
}

#[derive(Default)]
pub struct State;

impl State {
    pub fn new() -> Self {
        Self
    }

    pub fn update(&mut self, message: Message, ctx: &Context<'_>) -> iced::Task<Message> {
        match message {
            Message::ToggleGamescope(enabled) => {
                if let (Some(bottle), Some(state)) = (ctx.bottle.clone(), ctx.bottle_state) {
                    let mut config = state.wrappers().gamescope.clone();
                    config.enabled = enabled;

                    return iced::Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_gamescope(config);
                            edit.commit().await.map_err(|err| err.to_string())
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::ToggleMangoHud(enabled) => {
                if let Some(bottle) = ctx.bottle.clone() {
                    let config = MangoHudConfig { enabled };

                    return iced::Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_mangohud(config);
                            edit.commit().await.map_err(|err| err.to_string())
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::WrapperUpdated(Ok(())) => {}
            Message::WrapperUpdated(Err(err)) => eprintln!("failed to update settings: {err}"),
        }

        iced::Task::none()
    }

    pub fn view<'a>(&self, ctx: &Context<'a>) -> Element<'a, Message> {
        let Some(state) = ctx.bottle_state else {
            return column![].into();
        };
        let wrappers = state.wrappers();
        let environment_count = state.environment().iter().count();
        let environment_label = if environment_count == 0 {
            "None set".to_string()
        } else {
            format!("{environment_count} variables set")
        };

        let bottle = RowGroup::new()
            .title("Bottle")
            .add(
                ActionRow::new(state.runner().name(), ActionRowState::Disabled)
                    .description(state.runner().version())
                    .icon(Icon::Run),
            )
            .add(environment_row(environment_label));

        let graphics = RowGroup::new()
            .title("Graphics")
            .add(SwitcherRow::new("DLSS", false).description("Deep Learning Super Sampling"))
            .add(SwitcherRow::new("vkBasalt", false).description("Add post-processing effects"))
            .add(
                SwitcherRow::new("Discrete GPU", false).description("Force use your dedicated GPU"),
            )
            .add(
                SwitcherRow::new("Gamescope", wrappers.gamescope.enabled)
                    .description("Use the SteamOS compositor")
                    .on_toggle(Message::ToggleGamescope),
            )
            .add(
                SwitcherRow::new("MangoHud", wrappers.mangohud.enabled)
                    .description("Show a performance overlay")
                    .on_toggle(Message::ToggleMangoHud),
            )
            .add(PickerRow::new("Display Settings").description("Resolution and other options"));

        container(column![bottle, graphics].spacing(12))
            .max_width(1150)
            .into()
    }
}

fn environment_row(description: String) -> ListRow<'static, Message> {
    let labels = column![
        text("Environment variables").label(),
        text(description).detail().muted(),
    ]
    .spacing(6);

    ListRow::new(labels).leading(
        svg(Icon::Gear.handle())
            .width(24)
            .height(24)
            .content_fit(iced::ContentFit::Contain),
    )
}
