#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use bottles_core::{MangoHudConfig, error::Error as CoreError};
use iced::Task;
#[cfg(target_os = "linux")]
use uuid::Uuid;

use crate::domain::DomainSnapshots;

#[derive(Clone)]
pub enum Message {
    #[cfg(target_os = "linux")]
    ToggleGamescope { bottle_id: Uuid, enabled: bool },
    #[cfg(target_os = "linux")]
    ToggleMangoHud { bottle_id: Uuid, enabled: bool },
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
    pub fn update(&mut self, message: Message, domain: &DomainSnapshots) -> Task<Message> {
        match message {
            Message::ToggleGamescope { bottle_id, enabled } => {
                if !self.pending {
                    let Some(snapshot) = domain.bottle(bottle_id) else {
                        return Task::none();
                    };
                    let bottle = snapshot.bottle.clone();
                    let mut config = snapshot.state.wrappers().gamescope.clone();
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
            Message::ToggleMangoHud { bottle_id, enabled } => {
                if !self.pending {
                    let Some(snapshot) = domain.bottle(bottle_id) else {
                        return Task::none();
                    };
                    let bottle = snapshot.bottle.clone();
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
                self.finish_update(result);
            }
        }

        Task::none()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn update(&mut self, message: Message, _domain: &DomainSnapshots) -> Task<Message> {
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

    #[cfg(target_os = "linux")]
    fn finish_update(&mut self, result: Result<(), Arc<CoreError>>) {
        self.last_error = result.err().map(|error| error.to_string());
        self.pending = false;
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::State;

    #[test]
    fn completion_clears_pending_without_bottle_context() {
        let mut state = State {
            pending: true,
            last_error: None,
        };

        state.finish_update(Ok(()));

        assert!(!state.pending);
    }
}
