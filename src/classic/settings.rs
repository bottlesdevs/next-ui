//! Bottle "Settings" detail tab: platform-available wrapper toggles and
//! bottle/runner info. Bottle data comes from the selected core snapshot;
//! this state retains only local request/error state.

use std::sync::Arc;

use bottles_core::BottleState;
#[cfg(target_os = "linux")]
use bottles_core::{Bottle, MangoHudConfig, error::Error as CoreError};
use iced::{
    Element,
    widget::{column, container, svg, text},
};

#[cfg(target_os = "linux")]
use crate::widgets::info_card::{InfoCard, Kind};
use crate::{
    icons::Icon,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        list_row::ListRow,
        picker_row::PickerRow,
        row_group::RowGroup,
        switcher_row::SwitcherRow,
        text::TextExt as _,
    },
};

pub struct Context<'a> {
    #[cfg(target_os = "linux")]
    pub bottle: Option<Bottle>,
    pub bottle_state: Option<&'a Arc<BottleState>>,
}

#[derive(Clone)]
pub enum Message {
    #[cfg(target_os = "linux")]
    ToggleGamescope(bool),
    #[cfg(target_os = "linux")]
    ToggleMangoHud(bool),
    #[cfg(target_os = "linux")]
    WrapperUpdated {
        generation: u64,
        result: Result<(), Arc<CoreError>>,
    },
}

#[derive(Default)]
pub struct State {
    #[cfg(target_os = "linux")]
    generation: u64,
    #[cfg(target_os = "linux")]
    pending: bool,
    #[cfg(target_os = "linux")]
    last_error: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "linux")]
    pub fn update(&mut self, message: Message, ctx: &Context<'_>) -> iced::Task<Message> {
        match message {
            Message::ToggleGamescope(enabled) => {
                if !self.pending
                    && let (Some(bottle), Some(state)) = (ctx.bottle.clone(), ctx.bottle_state)
                {
                    let mut config = state.wrappers().gamescope.clone();
                    config.enabled = enabled;
                    let generation = self.next_generation();

                    return iced::Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_gamescope(config);
                            edit.commit().await.map_err(Arc::new)
                        },
                        move |result| Message::WrapperUpdated { generation, result },
                    );
                }
            }
            Message::ToggleMangoHud(enabled) => {
                if !self.pending
                    && let Some(bottle) = ctx.bottle.clone()
                {
                    let config = MangoHudConfig { enabled };
                    let generation = self.next_generation();

                    return iced::Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_mangohud(config);
                            edit.commit().await.map_err(Arc::new)
                        },
                        move |result| Message::WrapperUpdated { generation, result },
                    );
                }
            }
            Message::WrapperUpdated { generation, result } if generation == self.generation => {
                self.last_error = result.err().map(|error| error.to_string());
                self.pending = false;
            }
            Message::WrapperUpdated { .. } => {}
        }

        iced::Task::none()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn update(&mut self, message: Message, _ctx: &Context<'_>) -> iced::Task<Message> {
        match message {}
    }

    pub fn view<'a>(&'a self, ctx: &Context<'a>) -> Element<'a, Message> {
        let Some(state) = ctx.bottle_state else {
            return column![].into();
        };
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
            );

        #[cfg(target_os = "linux")]
        let graphics = {
            let wrappers = state.wrappers();
            graphics
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
        };

        let graphics = graphics
            .add(PickerRow::new("Display Settings").description("Resolution and other options"));

        let content = column![bottle, graphics].spacing(12);
        #[cfg(target_os = "linux")]
        let content = if let Some(error) = &self.last_error {
            content.push(InfoCard::new(
                Kind::Error,
                "Could not update bottle settings",
                error,
            ))
        } else {
            content
        };

        container(content).max_width(1150).into()
    }

    #[cfg(target_os = "linux")]
    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = true;
        self.last_error = None;
        self.generation
    }

    pub fn has_active_operation(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.pending
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

fn environment_row(description: String) -> ListRow<'static, Message> {
    let labels = column![
        text("Environment variables").label().medium(),
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
