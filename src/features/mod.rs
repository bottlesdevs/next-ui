pub(crate) mod bottle_settings;
pub(crate) mod bottles;
pub(crate) mod library;
pub(crate) mod profiles;
#[cfg(feature = "fvs")]
pub(crate) mod snapshots;

use bottles_core::Bottles;
use iced::Task;

use crate::domain::DomainSnapshots;

#[derive(Clone)]
pub(crate) enum Message {
    Bottles(bottles::Message),
    Settings(bottle_settings::Message),
    #[cfg(feature = "fvs")]
    Snapshots(snapshots::Message),
    Library(library::Message),
    Profiles(profiles::Message),
}

pub(crate) enum Output {
    BottleCreated,
}

/// Disposable workflow state shared by application presentations.
pub(crate) struct State {
    pub(crate) bottles: bottles::State,
    pub(crate) profiles: profiles::State,
    pub(crate) library: library::State,
    pub(crate) settings: bottle_settings::State,
    #[cfg(feature = "fvs")]
    pub(crate) snapshots: snapshots::State,
}

impl State {
    pub(crate) fn new(core: &Bottles, domain: &DomainSnapshots) -> Self {
        Self {
            bottles: bottles::State::new(core.bottles().clone(), core.addons()),
            profiles: profiles::State::new(core.profiles().clone(), domain.profiles().selected()),
            library: library::State::new(core.library().clone()),
            settings: bottle_settings::State::default(),
            #[cfg(feature = "fvs")]
            snapshots: snapshots::State::default(),
        }
    }

    pub(crate) fn has_active_operations(&self) -> bool {
        let active = self.bottles.has_active_operation()
            || self.profiles.has_active_operation()
            || self.settings.has_active_operation()
            || self.library.has_active_operation();
        #[cfg(feature = "fvs")]
        let active = active || self.snapshots.has_active_operation();
        active
    }

    pub(crate) fn cancel_active_operations(&mut self) {
        self.profiles.dismiss_dialogs();
        self.bottles.cancel_creation();
        self.library.cancel_active_operations();
        #[cfg(feature = "fvs")]
        self.snapshots.cancel_active_operations();
    }

    pub(crate) fn reload_library(&mut self) -> Task<Message> {
        self.library.reload().map(Message::Library)
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        domain: &DomainSnapshots,
    ) -> (Task<Message>, Option<Output>) {
        match message {
            Message::Bottles(message) => {
                let (task, output) = self.bottles.update(message);
                (
                    task.map(Message::Bottles),
                    output.map(|bottles::Output::Created| Output::BottleCreated),
                )
            }
            Message::Settings(message) => (
                self.settings.update(message, domain).map(Message::Settings),
                None,
            ),
            #[cfg(feature = "fvs")]
            Message::Snapshots(message) => {
                (self.snapshots.update(message).map(Message::Snapshots), None)
            }
            Message::Library(message) => (self.library.update(message).map(Message::Library), None),
            Message::Profiles(message) => (
                self.profiles
                    .update(message, domain.profiles().selected())
                    .map(Message::Profiles),
                None,
            ),
        }
    }

    pub(crate) fn profiles_changed(
        &mut self,
        domain: &DomainSnapshots,
        selected_changed: bool,
    ) -> Task<Message> {
        if selected_changed {
            self.profiles.sync_selected(domain.profiles().selected());
        }
        self.reload_library()
    }
}
