//! Storefront/Steam account linking: the "Link a storefront account" and
//! "Link Steam account" popovers, Steam candidate detection, the sign-in
//! modal (browser redirect / OAuth code entry), and unlinking. Reads
//! `active_profile` and the full profile list via [`Context`] from the
//! shell (needed to check whether a detected Steam account is already
//! linked to a different profile) and needs the shell-owned
//! `ProfileManagerHandle` to perform account/Steam mutations, since the
//! profile manager itself is owned by `features::profiles`.

use next_proto::bottles::{
    common::v1::{AuthState, Storefront},
    profiles::v1::{LinkAccountRequest, SteamLink, UserProfile, profile_client::ProfileClient},
    registry::v1::{ResolvePluginRequest, registry_client::RegistryClient},
    store::v1::{BeginLoginRequest, login_challenge, store_client::StoreClient},
};

use crate::{
    widgets::{
        button::{Button, ButtonKind},
        info_row::InfoRow,
        list_row::ListRow,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        row_group::RowGroup,
        text::TextExt as _,
        text_row::TextRow,
        title::Title,
    },
    features::profiles::ProfileManagerHandle,
    icons::Icon,
    theme,
};

const SERVER_ENDPOINT: &str = "http://127.0.0.1:50052";
const REGISTRY_ENDPOINT: &str = "http://127.0.0.1:50250";

const STOREFRONTS: &[Storefront] = &[
    Storefront::Steam,
    Storefront::EpicGames,
    Storefront::Gog,
    Storefront::AmazonGames,
    Storefront::EaApp,
    Storefront::UbisoftConnect,
    Storefront::BattleNet,
];
const LOGIN_STOREFRONTS: &[Storefront] = &[Storefront::EpicGames, Storefront::Gog];

/// Read-mostly data this feature needs from the shell — `active_profile`
/// and `profiles` are owned by `features::profiles`, and the manager
/// handle is needed to issue account/Steam mutations.
pub struct Context<'a> {
    pub active_profile: Option<&'a UserProfile>,
    pub profiles: &'a [UserProfile],
    pub profile_manager: Option<ProfileManagerHandle>,
}

/// Which of the two storefront-picker popovers on the Profiles pane is
/// open, if any — `ToggleLink`/`ToggleSteam` are mutually exclusive.
#[derive(Clone, PartialEq, Eq)]
enum LinkPopover {
    Closed,
    Storefront,
    Steam,
}

/// The storefront sign-in modal, if any.
#[derive(Clone, PartialEq, Eq)]
struct LoginChallenge {
    storefront: Storefront,
    challenge_id: String,
    url: String,
    code_draft: String,
    submitting: bool,
}

#[derive(Clone)]
pub enum Message {
    ToggleLink,
    ToggleSteam,
    SteamCandidatesDetected(Vec<bottles_core::steam::SteamUser>),
    Dismiss,
    UnlinkAccount(i32),
    BeginLogin(Storefront),
    LoginChallengeReceived(Result<(Storefront, String, String), String>),
    LoginCodeChanged(String),
    OpenLoginUrl,
    CopyLoginUrl,
    SubmitLoginCode,
    CancelLogin,
    UnlinkSteam,
    LinkSteam(String, String),
    ProfileUpdated(Result<UserProfile, String>),
    Noop,
}

pub struct State {
    link_popover: LinkPopover,
    steam_candidates: Vec<bottles_core::steam::SteamUser>,
    login_modal: Option<LoginChallenge>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            link_popover: LinkPopover::Closed,
            steam_candidates: Vec::new(),
            login_modal: None,
        }
    }

    /// Whether the sign-in modal is open — the shell needs this to decide
    /// whether opening the new-profile dialog should close it (the two
    /// overlays are mutually exclusive).
    pub fn login_open(&self) -> bool {
        self.login_modal.is_some()
    }

    pub fn close_login(&mut self) {
        self.login_modal = None;
    }

    pub fn update(&mut self, message: Message, ctx: &Context<'_>) -> iced::Task<Message> {
        match message {
            Message::ToggleLink => {
                self.link_popover = match self.link_popover {
                    LinkPopover::Storefront => LinkPopover::Closed,
                    LinkPopover::Closed | LinkPopover::Steam => LinkPopover::Storefront,
                };
            }
            Message::ToggleSteam => {
                self.link_popover = match self.link_popover {
                    LinkPopover::Steam => LinkPopover::Closed,
                    LinkPopover::Closed | LinkPopover::Storefront => LinkPopover::Steam,
                };

                if self.link_popover == LinkPopover::Steam {
                    return iced::Task::perform(
                        async { detect_steam_users() },
                        Message::SteamCandidatesDetected,
                    );
                }
            }
            Message::SteamCandidatesDetected(candidates) => self.steam_candidates = candidates,
            Message::Dismiss => self.link_popover = LinkPopover::Closed,
            Message::UnlinkAccount(storefront) => {
                if let (Some(handle), Some(active)) =
                    (ctx.profile_manager.clone(), ctx.active_profile.cloned())
                {
                    return iced::Task::perform(
                        async move {
                            handle
                                .manager()
                                .unlink_account(&active.id, storefront)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::BeginLogin(storefront) => {
                self.link_popover = LinkPopover::Closed;

                if let Some(active) = ctx.active_profile.cloned() {
                    return iced::Task::perform(
                        async move {
                            begin_login(active.id, storefront)
                                .await
                                .map(|(challenge_id, url)| (storefront, challenge_id, url))
                        },
                        Message::LoginChallengeReceived,
                    );
                }
            }
            Message::LoginChallengeReceived(Ok((storefront, challenge_id, url))) => {
                self.login_modal = Some(LoginChallenge {
                    storefront,
                    challenge_id,
                    url,
                    code_draft: String::new(),
                    submitting: false,
                });
            }
            Message::LoginChallengeReceived(Err(err)) => eprintln!("failed to start login: {err}"),
            Message::LoginCodeChanged(code) => {
                if let Some(login) = &mut self.login_modal {
                    login.code_draft = code;
                }
            }
            Message::OpenLoginUrl => {
                if let Some(login) = &self.login_modal {
                    open_url(&login.url);
                }
            }
            Message::CopyLoginUrl => {
                if let Some(login) = &self.login_modal {
                    return iced::clipboard::write(login.url.clone());
                }
            }
            Message::CancelLogin => self.login_modal = None,
            Message::SubmitLoginCode => {
                if let (Some(active), Some(login)) =
                    (ctx.active_profile.cloned(), &mut self.login_modal)
                {
                    login.submitting = true;

                    let profile_id = active.id;
                    let challenge_id = login.challenge_id.clone();
                    let storefront = login.storefront;
                    let user_input = login.code_draft.clone();

                    return iced::Task::perform(
                        async move {
                            complete_login(profile_id, challenge_id, storefront, user_input).await
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::UnlinkSteam => {
                if let (Some(handle), Some(active)) =
                    (ctx.profile_manager.clone(), ctx.active_profile.cloned())
                {
                    return iced::Task::perform(
                        async move {
                            handle
                                .manager()
                                .unlink_steam(&active.id)
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::LinkSteam(steam_id64, account_name) => {
                self.link_popover = LinkPopover::Closed;

                if let (Some(handle), Some(active)) =
                    (ctx.profile_manager.clone(), ctx.active_profile.cloned())
                {
                    return iced::Task::perform(
                        async move {
                            handle
                                .manager()
                                .link_steam(
                                    &active.id,
                                    SteamLink {
                                        steam_id64,
                                        account_name,
                                    },
                                )
                                .await
                                .map_err(|err| err.to_string())
                        },
                        Message::ProfileUpdated,
                    );
                }
            }
            Message::ProfileUpdated(Ok(_)) => {
                self.login_modal = None;
            }
            Message::ProfileUpdated(Err(err)) => {
                eprintln!("failed to update profile: {err}");

                if let Some(login) = &mut self.login_modal {
                    login.submitting = false;
                }
            }
            Message::Noop => {}
        }

        iced::Task::none()
    }

    /// Renders the "Linked accounts" / storefront-link / Steam-link
    /// sections for the profile settings page. Returns an empty column
    /// when there's no active profile.
    pub fn view_links<'a>(&'a self, ctx: &Context<'a>) -> iced::Element<'a, Message> {
        use iced::widget::{column, container, text};

        let Some(active) = ctx.active_profile else {
            return column![].into();
        };

        let mut accounts = RowGroup::new().title("Linked accounts");

        for account in &active.accounts {
            let storefront = Storefront::try_from(account.storefront).unwrap_or_default();
            accounts = accounts.add(account_row(
                storefront_icon(storefront),
                format!(
                    "{} on {}",
                    account.account_display_name,
                    storefront_label(storefront)
                ),
                auth_state_label(account.auth_state),
                Message::UnlinkAccount(account.storefront),
            ));
        }

        let link_trigger = PickerRow::new("Link a storefront account")
            .description("Choose the account to connect")
            .on_press(Message::ToggleLink);
        let mut link_popover = Popover::new(link_trigger, self.link_popover == LinkPopover::Storefront)
            .on_dismiss(Message::Dismiss)
            .footer("Not listed, install manually", Message::Noop);

        for storefront in STOREFRONTS {
            if active
                .accounts
                .iter()
                .any(|account| account.storefront == *storefront as i32)
            {
                continue;
            }

            let mut item =
                PopoverItem::new(storefront_label(*storefront)).icon(storefront_icon(*storefront));

            item = if LOGIN_STOREFRONTS.contains(storefront) {
                item.action("Link", Message::BeginLogin(*storefront))
            } else {
                item.subtitle("Coming soon")
            };

            link_popover = link_popover.add(item);
        }

        let steam_row: iced::Element<'_, Message> = if let Some(link) = &active.steam_link {
            account_row(
                Icon::Computer,
                &link.account_name,
                "Linked Steam account",
                Message::UnlinkSteam,
            )
            .into()
        } else {
            let steam_trigger = PickerRow::new("Link Steam account")
                .description("Detected from your local Steam installation")
                .on_press(Message::ToggleSteam);
            let mut steam_popover =
                Popover::new(steam_trigger, self.link_popover == LinkPopover::Steam)
                    .on_dismiss(Message::Dismiss);

            for user in &self.steam_candidates {
                let taken_by = ctx.profiles.iter().find(|profile| {
                    profile.id != active.id
                        && profile
                            .steam_link
                            .as_ref()
                            .is_some_and(|link| link.steam_id64 == user.steam_id64)
                });

                let mut item = PopoverItem::new(&user.account_name).icon(Icon::Computer);

                item = if let Some(owner) = taken_by {
                    item.disabled_action("Taken").tooltip(
                        column![
                            text("Already linked").detail(),
                            text(&owner.name).detail().muted(),
                        ]
                        .spacing(2),
                    )
                } else {
                    item.action(
                        "Link",
                        Message::LinkSteam(user.steam_id64.clone(), user.account_name.clone()),
                    )
                };

                steam_popover = steam_popover.add(item);
            }

            steam_popover.into()
        };

        column![
            accounts,
            container(link_popover).width(iced::Fill),
            container(steam_row).width(iced::Fill),
        ]
        .spacing(18)
        .into()
    }

    /// The sign-in modal's content, if the login flow is in progress. The
    /// shell composes this atop the settings page since it owns the modal
    /// stacking/dismiss chrome.
    pub fn login_dialog(&self) -> Option<iced::Element<'_, Message>> {
        self.login_modal.as_ref().map(login_dialog)
    }
}

fn login_dialog(login: &LoginChallenge) -> iced::Element<'_, Message> {
    use iced::widget::{column, container, row};

    let submit_label = if login.submitting {
        "Submitting…"
    } else {
        "Submit"
    };

    container(
        column![
            Title::new("Sign in").subtitle(storefront_label(login.storefront)),
            RowGroup::new()
                .add(
                    ListRow::from(
                        InfoRow::new("Sign-in link (click to copy)")
                            .description(&login.url)
                            .icon(storefront_icon(login.storefront)),
                    )
                    .on_press(Message::CopyLoginUrl),
                )
                .add(action_button_row(
                    Icon::Arrow,
                    "Open in your browser",
                    "Sign in there, then paste the code you're given below.",
                    "Open",
                    Message::OpenLoginUrl,
                ))
                .add(
                    TextRow::new("Authorization code", &login.code_draft)
                        .icon(Icon::Checkmark)
                        .on_input(Message::LoginCodeChanged)
                        .on_submit(Message::SubmitLoginCode),
                ),
            row![
                Button::new(submit_label)
                    .kind(ButtonKind::Primary)
                    .on_press_maybe((!login.submitting).then_some(Message::SubmitLoginCode)),
                Button::new("Cancel")
                    .kind(ButtonKind::Transparent)
                    .on_press(Message::CancelLogin),
            ]
            .spacing(12),
        ]
        .spacing(18),
    )
    .width(560)
    .padding(24)
    .style(theme::panel)
    .into()
}

pub fn account_row<'a>(
    icon: Icon,
    title: impl iced::widget::text::IntoFragment<'a>,
    description: &'a str,
    on_unlink: Message,
) -> ListRow<'a, Message> {
    action_button_row(icon, title, description, "Unlink", on_unlink)
}

pub fn action_button_row<'a, M: Clone + 'a>(
    icon: Icon,
    title: impl iced::widget::text::IntoFragment<'a>,
    description: &'a str,
    button_label: &'a str,
    on_press: M,
) -> ListRow<'a, M> {
    use iced::widget::{column, svg, text};

    let labels = column![text(title).label(), text(description).detail().muted()].spacing(6);

    ListRow::new(labels)
        .leading(
            svg(icon.handle())
                .width(24)
                .height(24)
                .content_fit(iced::ContentFit::Contain),
        )
        .trailing(
            Button::new(button_label)
                .kind(ButtonKind::Surface)
                .on_press(on_press),
        )
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

fn detect_steam_users() -> Vec<bottles_core::steam::SteamUser> {
    bottles_core::steam::loginusers_vdf_path()
        .and_then(|path| bottles_core::steam::parse_loginusers(&path).ok())
        .unwrap_or_default()
}

fn auth_state_label(state: i32) -> &'static str {
    match AuthState::try_from(state).unwrap_or_default() {
        AuthState::Active => "Connected",
        AuthState::Stale => "Needs re-authentication",
        AuthState::Inactive => "Signed out",
        AuthState::Unspecified => "Unknown",
    }
}

fn storefront_label(storefront: Storefront) -> &'static str {
    match storefront {
        Storefront::Steam => "Steam",
        Storefront::EpicGames => "Epic Games Store",
        Storefront::Gog => "GOG",
        Storefront::AmazonGames => "Amazon Games",
        Storefront::EaApp => "EA App",
        Storefront::UbisoftConnect => "Ubisoft Connect",
        Storefront::BattleNet => "Battle.net",
        Storefront::Unspecified => "Unknown storefront",
    }
}

fn storefront_icon(storefront: Storefront) -> Icon {
    match storefront {
        Storefront::Steam => Icon::Computer,
        Storefront::EpicGames | Storefront::Gog | Storefront::AmazonGames => Icon::Disk,
        Storefront::EaApp | Storefront::UbisoftConnect | Storefront::BattleNet => Icon::Controller,
        Storefront::Unspecified => Icon::Warning,
    }
}

async fn begin_login(
    profile_id: String,
    storefront: Storefront,
) -> Result<(String, String), String> {
    let mut registry = RegistryClient::connect(REGISTRY_ENDPOINT)
        .await
        .map_err(|err| format!("plugin registry unavailable: {err}"))?;
    let resolved = registry
        .resolve_plugin(ResolvePluginRequest {
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();
    let endpoint = resolved
        .endpoint
        .ok_or_else(|| format!("no {} plugin is running", storefront_label(storefront)))?;
    let mut store = StoreClient::connect(endpoint)
        .await
        .map_err(|err| err.to_string())?;
    let challenge = store
        .begin_login(BeginLoginRequest {
            profile_id,
            storefront: storefront as i32,
        })
        .await
        .map_err(|err| err.to_string())?
        .into_inner();

    if let Some(error) = challenge.error {
        return Err(error);
    }

    let url = match challenge.kind {
        Some(login_challenge::Kind::BrowserRedirect(challenge)) => challenge.url,
        Some(login_challenge::Kind::OauthRedirect(challenge)) => challenge.auth_url,
        _ => return Err("this storefront's login flow isn't supported yet".into()),
    };

    Ok((challenge.challenge_id, url))
}

async fn complete_login(
    profile_id: String,
    challenge_id: String,
    storefront: Storefront,
    user_input: String,
) -> Result<UserProfile, String> {
    let mut client = ProfileClient::connect(SERVER_ENDPOINT)
        .await
        .map_err(|err| format!("next-server unavailable: {err}"))?;

    client
        .link_account(LinkAccountRequest {
            profile_id,
            challenge_id,
            storefront: storefront as i32,
            user_input,
        })
        .await
        .map(|response| response.into_inner())
        .map_err(|err| err.to_string())
}
