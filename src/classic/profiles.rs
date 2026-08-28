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
        dialog::Dialog,
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
    OpenCreate,
    CreateNameChanged(String),
    SubmitCreate,
    DismissCreate,
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
}

struct NewProfileDialog {
    name: String,
    error: Option<String>,
}

impl NewProfileDialog {
    fn new() -> Self {
        Self {
            name: String::new(),
            error: None,
        }
    }

    fn view(&self, pending: bool) -> iced::widget::Column<'_, Message> {
        let mut content = column![
            container(Title::new("New profile").subtitle("Give this profile a name."))
                .center_x(iced::Fill),
            TextRow::new("Profile name", &self.name)
                .icon(Icon::Person)
                .on_input(Message::CreateNameChanged)
                .on_submit(Message::SubmitCreate),
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
                    .on_press(Message::SubmitCreate)
                    .loading(pending),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::DismissCreate),
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
    new_profile: Option<NewProfileDialog>,
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
            new_profile: None,
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

    pub(super) fn dialog(&self) -> Option<Dialog<'_, Message>> {
        self.new_profile.as_ref().map(|dialog| {
            Dialog::new(
                dialog.view(self.request_kind == Some(RequestKind::Create)),
                Message::DismissCreate,
            )
        })
    }

    pub(super) fn dismiss_dialog(&mut self) {
        self.new_profile = None;
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Output>) {
        let mut output = None;
        match message {
            Message::ToggleProfileSettings => {
                output = Some(Output::ToggleSettings);
            }
            Message::OpenCreate => {
                if self.request_kind.is_none() && self.new_profile.is_none() {
                    self.new_profile = Some(NewProfileDialog::new());
                }
            }
            Message::CreateNameChanged(name) => {
                if let Some(dialog) = &mut self.new_profile {
                    dialog.name = name;
                }
            }
            Message::DismissCreate => self.dismiss_dialog(),
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
            Message::SubmitCreate => {
                if self.request_kind.is_some() {
                    return (Task::none(), None);
                }
                let Some(dialog) = &mut self.new_profile else {
                    return (Task::none(), None);
                };

                let profiles = self.profiles.clone();
                dialog.error = None;
                let name = if dialog.name.trim().is_empty() {
                    "New profile".to_owned()
                } else {
                    dialog.name.trim().to_owned()
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
                    self.new_profile = None;
                }
                self.request_kind = None;
            }
            Message::ProfileUpdated {
                generation,
                result: Err(error),
            } if generation == self.request_generation => {
                if self.request_kind == Some(RequestKind::Create) {
                    if let Some(dialog) = &mut self.new_profile {
                        dialog.error = Some(error.to_string());
                    }
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
        dialog.name = "Player one".into();

        assert_eq!(dialog.name, "Player one");
        dialog.error = Some("creation failed".into());
        assert_eq!(dialog.name, "Player one");
        assert_eq!(dialog.error.as_deref(), Some("creation failed"));
    }
}
