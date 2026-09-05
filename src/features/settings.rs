use std::sync::Arc;

use bottles_core::BottleState;
#[cfg(target_os = "linux")]
use bottles_core::{Bottle, MangoHudConfig, error::Error as CoreError};
use iced::Task;

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
    #[cfg(target_os = "linux")]
    pub fn update(&mut self, message: Message, ctx: &Context<'_>) -> Task<Message> {
        match message {
            Message::ToggleGamescope(enabled) => {
                if !self.pending
                    && let (Some(bottle), Some(state)) = (ctx.bottle.clone(), ctx.bottle_state)
                {
                    let mut config = state.wrappers().gamescope.clone();
                    config.enabled = enabled;
                    let generation = self.next_generation();

                    return Task::perform(
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
                    let generation = self.next_generation();
                    return Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_mangohud(MangoHudConfig { enabled });
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

        Task::none()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn update(&mut self, message: Message, _ctx: &Context<'_>) -> Task<Message> {
        match message {}
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

    #[cfg(target_os = "linux")]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    #[cfg(target_os = "linux")]
    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = true;
        self.last_error = None;
        self.generation
    }
}
