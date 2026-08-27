//! Profile management backed directly by `next-core`.

use std::sync::Arc;

use bottles_core::{Profile, Profiles, ProfilesConfig, error::Error as CoreError};
use iced::{
    Element, Task,
    widget::{column, container, row},
};
use uuid::Uuid;

use crate::{
    icons::Icon,
    widgets::{
        button::{Button, ButtonKind},
        info_card::{InfoCard, Kind as InfoCardKind},
        popover::{Popover, PopoverItem},
        text_row::TextRow,
        title::Title,
    },
};

pub fn profile_events(
    profiles: &Profiles,
) -> impl iced::futures::Stream<Item = Arc<ProfilesConfig>> + Send + 'static + use<> {
    profiles.watch()
}

#[derive(Clone)]
pub enum Message {
    ToggleProfileSettings,
    ActivateProfile(Uuid),
    Create(String),
    ProfileUpdated {
        generation: u64,
        result: Result<Profile, Arc<CoreError>>,
    },
    NameChanged(String),
    RenameSubmit,
    DeleteProfile(Uuid),
    ProfileDeleted {
        generation: u64,
        result: Result<(), Arc<CoreError>>,
    },
}

pub enum Output {
    ToggleSettings,
    CreateFinished(Result<(), Arc<CoreError>>),
}

#[derive(Clone)]
pub(crate) enum NewProfileMessage {
    NameChanged(String),
    Submit,
    Cancel,
}

pub(super) struct NewProfileDialog {
    name: String,
    error: Option<String>,
}

impl NewProfileDialog {
    pub(super) fn new() -> Self {
        Self {
            name: String::new(),
            error: None,
        }
    }

    pub(super) fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub(super) fn submission(&self) -> String {
        self.name.clone()
    }

    pub(super) fn clear_error(&mut self) {
        self.error = None;
    }

    pub(super) fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub(super) fn view(&self, pending: bool) -> iced::widget::Column<'_, NewProfileMessage> {
        let mut content = column![
            container(Title::new("New profile").subtitle("Give this profile a name."))
                .center_x(iced::Fill),
            TextRow::new("Profile name", &self.name)
                .icon(Icon::Person)
                .on_input(NewProfileMessage::NameChanged)
                .on_submit(NewProfileMessage::Submit),
        ]
        .spacing(18);

        if let Some(error) = &self.error {
            content = content.push(
                InfoCard::new(InfoCardKind::Error, "Could not create profile", error)
                    .width(iced::Fill),
            );
        }

        content.push(
            row![
                Button::new("Create")
                    .kind(ButtonKind::Primary)
                    .on_press(NewProfileMessage::Submit)
                    .loading(pending),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(NewProfileMessage::Cancel),
            ]
            .spacing(12),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestKind {
    Select,
    Create,
    Rename,
    Delete,
}

pub struct State {
    profiles: Profiles,
    selected_id: Uuid,
    name_draft: String,
    last_error: Option<String>,
    request_generation: u64,
    request_kind: Option<RequestKind>,
}

impl State {
    pub fn new(profiles: Profiles, selected: &Profile) -> Self {
        Self {
            profiles,
            selected_id: selected.id(),
            name_draft: selected.name().to_owned(),
            last_error: None,
            request_generation: 0,
            request_kind: None,
        }
    }

    /// Synchronizes local edit state with the Classic read model's selection.
    pub fn sync_selected(&mut self, selected: &Profile) {
        self.selected_id = selected.id();
        self.name_draft = selected.name().to_owned();
    }

    pub fn name_draft(&self) -> &str {
        &self.name_draft
    }

    pub(super) fn manager(&self) -> &Profiles {
        &self.profiles
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn has_active_operation(&self) -> bool {
        self.request_kind.is_some()
    }

    pub(super) fn creating_profile(&self) -> bool {
        self.request_kind == Some(RequestKind::Create)
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Output>) {
        let mut output = None;
        match message {
            Message::ToggleProfileSettings => {
                output = Some(Output::ToggleSettings);
            }
            Message::ActivateProfile(id) => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    let generation = self.begin_request(RequestKind::Select);
                    return (
                        Task::perform(
                            async move { profiles.select(id).await.map_err(Arc::new) },
                            move |result| Message::ProfileUpdated { generation, result },
                        ),
                        None,
                    );
                }
            }
            Message::Create(draft) => {
                if self.request_kind.is_some() {
                    return (Task::none(), None);
                }

                let profiles = self.profiles.clone();
                let name = if draft.trim().is_empty() {
                    "New profile".to_owned()
                } else {
                    draft.trim().to_owned()
                };

                let generation = self.begin_request(RequestKind::Create);
                return (
                    Task::perform(
                        async move { profiles.create(name).await.map_err(Arc::new) },
                        move |result| Message::ProfileUpdated { generation, result },
                    ),
                    None,
                );
            }
            Message::ProfileUpdated {
                generation,
                result: Ok(_),
            } if generation == self.request_generation => {
                self.last_error = None;
                if self.request_kind == Some(RequestKind::Create) {
                    output = Some(Output::CreateFinished(Ok(())));
                }
                self.request_kind = None;
            }
            Message::ProfileUpdated {
                generation,
                result: Err(error),
            } if generation == self.request_generation => {
                if self.request_kind == Some(RequestKind::Create) {
                    output = Some(Output::CreateFinished(Err(error)));
                } else {
                    self.last_error = Some(error.to_string());
                }
                self.request_kind = None;
            }
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    let selected_id = self.selected_id;
                    let name = self.name_draft.clone();
                    let generation = self.begin_request(RequestKind::Rename);
                    return (
                        Task::perform(
                            async move { profiles.rename(selected_id, name).await.map_err(Arc::new) },
                            move |result| Message::ProfileUpdated { generation, result },
                        ),
                        None,
                    );
                }
            }
            Message::DeleteProfile(id) => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    let generation = self.begin_request(RequestKind::Delete);
                    return (
                        Task::perform(
                            async move { profiles.delete(id).await.map_err(Arc::new) },
                            move |result| Message::ProfileDeleted { generation, result },
                        ),
                        None,
                    );
                }
            }
            Message::ProfileDeleted {
                generation,
                result: Err(error),
            } if generation == self.request_generation => {
                self.last_error = Some(error.to_string());
                self.request_kind = None;
            }
            Message::ProfileDeleted {
                generation,
                result: Ok(()),
            } if generation == self.request_generation => {
                self.last_error = None;
                self.request_kind = None;
            }
            Message::ProfileUpdated { .. } | Message::ProfileDeleted { .. } => {}
        }

        (Task::none(), output)
    }

    fn begin_request(&mut self, kind: RequestKind) -> u64 {
        self.request_generation = self.request_generation.wrapping_add(1);
        self.request_kind = Some(kind);
        self.last_error = None;
        self.request_generation
    }

    pub fn view_switcher<'a>(&self, snapshot: &'a ProfilesConfig) -> Element<'a, Message> {
        let selected = snapshot.selected();
        let label = selected.name();
        let trigger = Button::icon_only(label, Icon::Person)
            .diameter(32.0)
            .icon_size(16.0)
            .kind(ButtonKind::Transparent)
            .on_press(());

        let mut switcher = Popover::new(trigger).footer("Profiles", Message::ToggleProfileSettings);

        for profile in snapshot.profiles() {
            switcher = switcher.add(
                PopoverItem::new(profile.name())
                    .icon(Icon::Person)
                    .selected(selected.id() == profile.id())
                    .on_select(Message::ActivateProfile(profile.id())),
            );
        }

        switcher.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_submission_does_not_consume_the_dialog_draft() {
        let mut dialog = NewProfileDialog::new();
        dialog.set_name("Player one".into());

        assert_eq!(dialog.submission(), "Player one");
        dialog.set_error("creation failed".into());
        assert_eq!(dialog.submission(), "Player one");
        assert_eq!(dialog.error.as_deref(), Some("creation failed"));
    }
}
