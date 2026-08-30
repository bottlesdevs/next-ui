//! Bottle "Settings" detail tab: platform-available wrapper toggles and
//! bottle/runner info. Bottle data comes from the selected core snapshot;
//! this state retains only local request/error state.

use std::sync::Arc;

use bottles_core::BottleState;
#[cfg(target_os = "linux")]
use bottles_core::{Bottle, MangoHudConfig, error::Error as CoreError};
use iced::{
    Element, Length,
    widget::{column, responsive},
};

#[cfg(target_os = "linux")]
use crate::widgets::info_card::{InfoCard, Kind};
use crate::{
    icons::Icon,
    widgets::{
        action_row::{ActionRow, State as ActionRowState},
        cycle_row::CycleRow,
        expander_row::ExpanderRow,
        info_row::InfoRow,
        list_row::{self, ListRow},
        row_group::RowGroup,
        spacing,
        switcher_row::SwitcherRow,
    },
};

const SETTINGS_TRACK_MIN_WIDTH: f32 = 300.0;

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
        let content = column![
            responsive(move |size| {
                let columns =
                    usize::from(size.width >= SETTINGS_TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
                let bottle_name = ListRow::new(list_row::labels("Bottle Name", state.name()))
                    .trailing(
                        Icon::Pencil
                            .view()
                            .width(list_row::BODY_SIZE)
                            .height(list_row::BODY_SIZE),
                    )
                    .enabled(false);
                let unavailable = || {
                    InfoRow::new("Not available yet")
                        .description("This setting is not supported yet.")
                };
                let bottle = RowGroup::new()
                    .title("Bottle")
                    .columns(columns)
                    .row(bottle_name)
                    .expander(
                        ExpanderRow::with_header(
                            InfoRow::new("Runner")
                                .description(format!(
                                    "{} {}",
                                    state.runner().name(),
                                    state.runner().version()
                                ))
                                .icon(Icon::Run),
                        )
                        .add(unavailable())
                        .content_enabled(false),
                    )
                    .expander(
                        ExpanderRow::new("Dependencies")
                            .description("Install fonts, codecs, libraries...")
                            .add(unavailable())
                            .content_enabled(false),
                    )
                    .expander(
                        ExpanderRow::new("Drives")
                            .description("Define your custom drives")
                            .add(unavailable())
                            .content_enabled(false),
                    );

                let graphics = RowGroup::new()
                    .title("Graphics")
                    .columns(columns)
                    .row(
                        SwitcherRow::new("DLSS", false).description("Deep Learning Super Sampling"),
                    )
                    .row(
                        SwitcherRow::new("vkBasalt", false)
                            .description("Add post-processing effects"),
                    )
                    .row(
                        SwitcherRow::new("Discrete GPU", false)
                            .description("Force use your dedicated GPU"),
                    )
                    .expander(
                        ExpanderRow::with_header(
                            SwitcherRow::new("FSR", false)
                                .description("FidelityFX Super Resolution"),
                        )
                        .columns(2)
                        .add(
                            ActionRow::new("Quality", ActionRowState::Disabled)
                                .description("Balanced"),
                        )
                        .add(CycleRow::new("Sharpening", "5")),
                    );

                #[cfg(target_os = "linux")]
                let graphics = {
                    let wrappers = state.wrappers();
                    graphics.row(
                        SwitcherRow::new("Gamescope", wrappers.gamescope.enabled)
                            .description("Use the SteamOS compositor")
                            .on_toggle(Message::ToggleGamescope),
                    )
                };

                let graphics = graphics.expander(
                    ExpanderRow::with_header(
                        SwitcherRow::new("Display Settings", false)
                            .description("Resolution and other options"),
                    )
                    .add(unavailable())
                    .content_enabled(false),
                );

                #[cfg(target_os = "linux")]
                let graphics = {
                    let wrappers = state.wrappers();
                    graphics.row(
                        SwitcherRow::new("MangoHud", wrappers.mangohud.enabled)
                            .description("Show a performance overlay")
                            .on_toggle(Message::ToggleMangoHud),
                    )
                };

                column![bottle, graphics].spacing(spacing::SM).into()
            })
            .height(Length::Shrink)
        ];
        #[cfg(target_os = "linux")]
        let content = if let Some(error) = &self.last_error {
            content.push(
                InfoCard::new(Kind::Error, "Could not update bottle settings", error)
                    .width(iced::Fill),
            )
        } else {
            content
        };

        content.into()
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
