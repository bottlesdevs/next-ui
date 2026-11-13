//! The profile button in the header, its switching menu, and the Manage
//! Profiles side pane, which the shell owns.

use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, row, text},
};
use next_ui::{
    Icon, chrome,
    layout::PaneContext,
    widget::{
        ActionRow, Avatar, Button, ButtonKind, ListRow, Popover, PopoverItem, RowGroup,
        TextExt as _, TextRow, Title, spacing,
    },
};
use uuid::Uuid;

use crate::{
    app::Request,
    model::{Action, Model},
    parts,
    profiles::Profile,
};

/// Whether the pane is open, which profile it edits, and an unsaved name.
#[derive(Default)]
pub struct State {
    open: bool,
    editing: Option<Uuid>,
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Uuid),
    Open,
    Close,
    Edit(Uuid),
    New,
    Name(String),
    SaveName,
    Delete(Uuid),
    Unlink(Uuid),
    /// Closes a confirmation without doing anything.
    Dismiss,
    Window(chrome::Action),
}

impl State {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn update(&mut self, model: &Model, message: Message) -> Option<Request> {
        match message {
            Message::Select(id) => return Some(Request::Act(Action::SelectProfile(id))),
            Message::Open => {
                self.open = true;
                self.editing = Some(model.profiles.selected);
                self.name = None;
            }
            Message::Close => {
                self.open = false;
                self.name = None;
            }
            Message::Edit(id) => {
                self.editing = Some(id);
                self.name = None;
            }
            Message::New => {
                // The new profile becomes the selected one, so edit that.
                self.editing = None;
                self.name = None;
                return Some(Request::Act(Action::CreateProfile(
                    "New Profile".to_owned(),
                )));
            }
            Message::Name(name) => self.name = Some(name),
            Message::SaveName => {
                let name = self.name.take()?;
                let id = self.editing(model)?.id;
                return Some(Request::Act(Action::RenameProfile(id, name)));
            }
            Message::Delete(id) => {
                if self.editing == Some(id) {
                    self.editing = None;
                }
                return Some(Request::Act(Action::DeleteProfile(id)));
            }
            Message::Unlink(link) => return Some(Request::Act(Action::Unlink(link))),
            Message::Dismiss => {}
            Message::Window(action) => return Some(Request::Window(action)),
        }
        None
    }

    fn editing<'a>(&self, model: &'a Model) -> Option<&'a Profile> {
        self.editing
            .and_then(|id| model.profiles.profile(id))
            .or_else(|| Some(model.profiles.selected()))
    }

    /// The Manage Profiles side pane.
    pub fn pane<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button(
                    "Close profiles",
                    Icon::Arrow,
                    Message::Close,
                ))
                .middle(Title::new("Manage Profiles"))
        });
        let Some(editing) = self.editing(model) else {
            return parts::page(header, column![]);
        };
        let profiles = &model.profiles;
        let deletable = profiles.profiles.len() > 1;

        let list = profiles
            .profiles
            .iter()
            .map(|profile| -> Element<'a, Message> {
                let active = profile.id == profiles.selected;
                let delete: Element<'a, Message> = if deletable {
                    Popover::confirm(
                        next_ui::widget::header_button("Delete profile", Icon::Trash),
                        format!("Delete {}?", profile.name),
                        "Its linked accounts are removed from Bottles. Installed games and \
                     programs stay.",
                        "Delete",
                        Message::Delete(profile.id),
                        Message::Dismiss,
                    )
                    .into()
                } else {
                    Space::new().into()
                };
                ActionRow::new(profile.name.as_str(), Message::Edit(profile.id))
                    .description(summary(profile, active))
                    .leading(Avatar::new(initial(&profile.name)).active(active))
                    .trailing(delete)
                    .selected(profile.id == editing.id)
                    .into()
            });

        let name = self.name.as_deref().unwrap_or(&editing.name);
        let accounts = model.account_providers().into_iter().map(
            move |(provider, provider_name)| -> Element<'a, Message> {
                let link = editing
                    .accounts
                    .iter()
                    .find(|account| account.provider.id == provider);
                let (status, action) = match link {
                    Some(link) => (
                        link.identity.display_name.clone(),
                        Button::new("Unlink").on_press(Message::Unlink(link.link_id)),
                    ),
                    // Linking has no design yet, so the button stays disabled.
                    None => ("Not linked".to_owned(), Button::new("Link")),
                };
                ListRow::new(
                    column![
                        text(provider_name).label().medium(),
                        text(status).size(16).muted()
                    ]
                    .spacing(spacing::XS),
                )
                .trailing(action)
                .into()
            },
        );

        let body = column![
            row![
                text("Profiles").size(18).medium().width(Fill),
                parts::header_button("New profile", Icon::Plus, Message::New),
            ]
            .align_y(Alignment::Center),
            list.fold(RowGroup::new(), RowGroup::row),
            Space::new().height(spacing::SM),
            text(editing.name.as_str()).size(18).medium(),
            accounts.fold(
                RowGroup::new().row(
                    TextRow::new("Profile Name", name)
                        .on_input(Message::Name)
                        .on_submit(Message::SaveName),
                ),
                RowGroup::row,
            ),
        ]
        .spacing(spacing::SM);

        parts::page(header, body)
    }
}

/// The header button showing the selected profile, with its switching menu.
pub fn button(model: &Model) -> Element<'_, Message> {
    let profiles = &model.profiles;
    let selected = profiles.selected();
    let trigger = Button::custom(
        row![
            text(selected.name.as_str()).size(17).muted(),
            Avatar::new(initial(&selected.name)).size(32.0).active(true),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .kind(ButtonKind::Transparent);

    let mut menu = Popover::new(trigger).width(300.0);
    for profile in &profiles.profiles {
        let active = profile.id == profiles.selected;
        menu = menu.item(
            PopoverItem::new(profile.name.as_str())
                .subtitle(accounts(profile))
                .leading(Avatar::new(initial(&profile.name)).active(active))
                .selected(active)
                .on_select(Message::Select(profile.id)),
        );
    }
    // The icon takes the avatars' column, so the label lines up with the
    // profile names above it.
    menu.separator()
        .item(
            PopoverItem::new("Manage Profiles")
                .leading(iced::widget::container(Icon::People.view()).center(44))
                .on_select(Message::Open),
        )
        .into()
}

fn initial(name: &str) -> &str {
    name.char_indices()
        .nth(1)
        .map_or(name, |(end, _)| &name[..end])
}

fn accounts(profile: &Profile) -> &'static str {
    if profile.accounts.is_empty() {
        "No linked accounts"
    } else {
        "Linked accounts"
    }
}

fn summary(profile: &Profile, active: bool) -> String {
    let stores = profile
        .accounts
        .iter()
        .map(|account| account.provider.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    match (active, stores.is_empty()) {
        (true, true) => "Active".to_owned(),
        (true, false) => format!("Active · {stores}"),
        (false, true) => "No linked accounts".to_owned(),
        (false, false) => stores,
    }
}
