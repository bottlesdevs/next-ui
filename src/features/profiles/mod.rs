mod account_link;

use std::sync::Arc;

use bottles_core::{Profile, Profiles, StorefrontProvider, error::Error as CoreError};
use iced::Task;
use uuid::Uuid;

pub use account_link::{LoginDialog, Message as AccountMessage, State as AccountLinkState};

#[derive(Clone)]
pub enum Message {
    ActivateProfile(Uuid),
    OpenCreate,
    CreateNameChanged(String),
    SubmitCreate,
    DismissCreate,
    ProfileUpdated(Result<Profile, Arc<CoreError>>),
    NameChanged(String),
    RenameSubmit,
    DeleteProfile(Uuid),
    ProfileDeleted(Result<(), Arc<CoreError>>),
    Account(AccountMessage),
}

#[derive(Default)]
pub struct NewProfile {
    name: String,
    error: Option<String>,
}

impl NewProfile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
    new_profile: Option<NewProfile>,
    last_error: Option<String>,
    request_kind: Option<RequestKind>,
    account_link: AccountLinkState,
}

impl State {
    pub fn new(profiles: Profiles, selected: &Profile) -> Self {
        Self {
            profiles,
            selected_id: selected.id(),
            name_draft: selected.name().to_owned(),
            new_profile: None,
            last_error: None,
            request_kind: None,
            account_link: AccountLinkState::default(),
        }
    }

    pub fn sync_selected(&mut self, selected: &Profile) {
        if self.selected_id != selected.id() {
            self.account_link.cancel_active_operation();
        }
        self.selected_id = selected.id();
        self.name_draft = selected.name().to_owned();
    }

    pub fn account_providers(&self) -> Vec<StorefrontProvider> {
        self.profiles.account_providers()
    }

    pub fn name_draft(&self) -> &str {
        &self.name_draft
    }

    pub fn new_profile(&self) -> Option<&NewProfile> {
        self.new_profile.as_ref()
    }

    pub fn is_creating_profile(&self) -> bool {
        self.request_kind == Some(RequestKind::Create)
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn account_link(&self) -> &AccountLinkState {
        &self.account_link
    }

    pub fn has_active_operation(&self) -> bool {
        self.request_kind.is_some() || self.account_link.has_active_operation()
    }

    pub fn dismiss_dialogs(&mut self) {
        self.new_profile = None;
        self.account_link.cancel_active_operation();
    }

    pub fn update(&mut self, message: Message, active_profile: &Profile) -> Task<Message> {
        match message {
            Message::OpenCreate => {
                if self.request_kind.is_none() && self.new_profile.is_none() {
                    self.account_link.cancel_active_operation();
                    self.new_profile = Some(NewProfile::default());
                }
            }
            Message::CreateNameChanged(name) => {
                if let Some(dialog) = &mut self.new_profile {
                    dialog.name = name;
                }
            }
            Message::DismissCreate => self.new_profile = None,
            Message::ActivateProfile(id) => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    self.begin_request(RequestKind::Select);
                    return Task::perform(
                        async move { profiles.select(id).await.map_err(Arc::new) },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::SubmitCreate => {
                if self.request_kind.is_some() {
                    return Task::none();
                }
                let Some(dialog) = &mut self.new_profile else {
                    return Task::none();
                };

                let profiles = self.profiles.clone();
                dialog.error = None;
                let name = if dialog.name.trim().is_empty() {
                    "New profile".to_owned()
                } else {
                    dialog.name.trim().to_owned()
                };

                self.begin_request(RequestKind::Create);
                return Task::perform(
                    async move { profiles.create(name).await.map_err(Arc::new) },
                    Message::ProfileUpdated,
                );
            }
            Message::ProfileUpdated(result) => {
                let Some(kind) = self.request_kind else {
                    return Task::none();
                };
                if kind == RequestKind::Delete {
                    return Task::none();
                }
                self.request_kind = None;

                match result {
                    Ok(_) => {
                        self.last_error = None;
                        if kind == RequestKind::Create {
                            self.new_profile = None;
                        }
                    }
                    Err(error) if kind == RequestKind::Create => {
                        if let Some(dialog) = &mut self.new_profile {
                            dialog.error = Some(error.to_string());
                        }
                    }
                    Err(error) => self.last_error = Some(error.to_string()),
                }
            }
            Message::NameChanged(name) => self.name_draft = name,
            Message::RenameSubmit => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    let selected_id = self.selected_id;
                    let name = self.name_draft.clone();
                    self.begin_request(RequestKind::Rename);
                    return Task::perform(
                        async move { profiles.rename(selected_id, name).await.map_err(Arc::new) },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::DeleteProfile(id) => {
                if self.request_kind.is_none() {
                    let profiles = self.profiles.clone();
                    self.begin_request(RequestKind::Delete);
                    return Task::perform(
                        async move { profiles.delete(id).await.map_err(Arc::new) },
                        Message::ProfileDeleted,
                    );
                }
            }
            Message::ProfileDeleted(result) if self.request_kind == Some(RequestKind::Delete) => {
                self.request_kind = None;
                self.last_error = result.err().map(|error| error.to_string());
            }
            Message::ProfileDeleted(_) => {}
            Message::Account(message) => {
                if matches!(message, AccountMessage::BeginLogin(_)) {
                    self.new_profile = None;
                }
                return self
                    .account_link
                    .update(message, active_profile, &self.profiles)
                    .map(Message::Account);
            }
        }

        Task::none()
    }

    fn begin_request(&mut self, kind: RequestKind) {
        self.request_kind = Some(kind);
        self.last_error = None;
    }
}
