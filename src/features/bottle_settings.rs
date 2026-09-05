use std::sync::Arc;

use bottles_core::{Bottle, BottleState};
#[cfg(target_os = "linux")]
use bottles_core::{MangoHudConfig, error::Error as CoreError};
use iced::Task;

#[derive(Clone)]
pub enum Message {
    #[cfg(target_os = "linux")]
    ToggleGamescope(bool),
    #[cfg(target_os = "linux")]
    ToggleMangoHud(bool),
    #[cfg(target_os = "linux")]
    WrapperUpdated(Result<(), Arc<CoreError>>),
}

#[derive(Default)]
pub struct State {
    #[cfg(target_os = "linux")]
    pending: bool,
    #[cfg(target_os = "linux")]
    last_error: Option<String>,
}

impl State {
    #[cfg(target_os = "linux")]
    pub fn update(
        &mut self,
        message: Message,
        selected: Option<(Bottle, &BottleState)>,
    ) -> Task<Message> {
        match message {
            Message::ToggleGamescope(enabled) => {
                if !self.pending
                    && let Some((bottle, state)) = selected
                {
                    let mut config = state.wrappers().gamescope.clone();
                    config.enabled = enabled;
                    self.start_update();

                    return Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_gamescope(config);
                            edit.commit().await.map_err(Arc::new)
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::ToggleMangoHud(enabled) => {
                if !self.pending
                    && let Some((bottle, _)) = selected
                {
                    self.start_update();
                    return Task::perform(
                        async move {
                            let mut edit = bottle.edit();
                            edit.set_mangohud(MangoHudConfig { enabled });
                            edit.commit().await.map_err(Arc::new)
                        },
                        Message::WrapperUpdated,
                    );
                }
            }
            Message::WrapperUpdated(result) => {
                self.last_error = result.err().map(|error| error.to_string());
                self.pending = false;
            }
        }

        Task::none()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn update(
        &mut self,
        message: Message,
        _selected: Option<(Bottle, &BottleState)>,
    ) -> Task<Message> {
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
    fn start_update(&mut self) {
        self.pending = true;
        self.last_error = None;
    }
}
