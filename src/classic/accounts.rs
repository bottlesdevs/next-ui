//! Storefront account linking backed directly by `next-core` providers.

use std::sync::{Arc, Mutex};

use bottles_core::{
    AccountLinkInteraction, PluginId, Profile, Profiles, StorefrontProvider,
    error::Error as CoreError,
};
use iced::futures::channel::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    icons::Icon,
    theme,
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
};

pub struct Context<'a> {
    pub active_profile: &'a Profile,
    pub profiles: &'a Profiles,
}

#[derive(Clone)]
pub struct LoginPrompt {
    url: String,
    instructions: String,
    response: Arc<Mutex<Option<oneshot::Sender<String>>>>,
}

impl LoginPrompt {
    fn new(url: String, instructions: String) -> (Self, oneshot::Receiver<String>) {
        let (response, answer) = oneshot::channel();
        (
            Self {
                url,
                instructions,
                response: Arc::new(Mutex::new(Some(response))),
            },
            answer,
        )
    }

    fn submit(&self, input: impl Into<String>) -> Result<(), &'static str> {
        let response = self
            .response
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .ok_or("account-link prompt was already answered")?;
        response
            .send(input.into())
            .map_err(|_| "account-link session is closed")
    }

    fn cancel(&self) {
        let _ = self
            .response
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}

struct LoginInteraction {
    prompts: mpsc::UnboundedSender<LoginPrompt>,
}

#[async_trait::async_trait]
impl AccountLinkInteraction for LoginInteraction {
    async fn request_input(&self, url: url::Url, instructions: String) -> Result<String, String> {
        let (prompt, answer) = LoginPrompt::new(url.to_string(), instructions);
        self.prompts
            .unbounded_send(prompt)
            .map_err(|_| "account-link UI is closed".to_owned())?;
        answer
            .await
            .map_err(|_| "account linking was cancelled".to_owned())
    }
}

struct LoginChallenge {
    code_draft: String,
    prompt: LoginPrompt,
    submitting: bool,
    error: Option<String>,
}

impl LoginChallenge {
    fn cancel(self) {
        self.prompt.cancel();
    }
}

#[derive(Clone)]
pub enum Message {
    ToggleLink,
    Dismiss,
    UnlinkAccount(PluginId),
    BeginLogin(StorefrontProvider),
    LoginRequested(LoginPrompt),
    LoginCodeChanged(String),
    OpenLoginUrl,
    CopyLoginUrl,
    SubmitLoginCode,
    CancelLogin,
    ProfileUpdated(Result<Profile, Arc<CoreError>>),
    Noop,
}

pub enum Output {
    OpenDialog,
    CloseDialog,
}

pub struct State {
    link_popover_open: bool,
    providers: Vec<StorefrontProvider>,
    login_modal: Option<LoginChallenge>,
    link_cancellation: Option<CancellationToken>,
    mutation_pending: bool,
    last_error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            link_popover_open: false,
            providers: Vec::new(),
            login_modal: None,
            link_cancellation: None,
            mutation_pending: false,
            last_error: None,
        }
    }

    /// Requests cancellation without dropping the task that drives the session.
    pub fn cancel_active_operation(&mut self) {
        if let Some(cancellation) = &self.link_cancellation {
            cancellation.cancel();
        }
        if let Some(login) = self.login_modal.take() {
            login.cancel();
        }
    }

    pub fn close_login(&mut self) {
        self.cancel_active_operation();
    }

    pub fn has_active_operation(&self) -> bool {
        self.link_cancellation.is_some() || self.mutation_pending
    }

    pub fn update(
        &mut self,
        message: Message,
        ctx: &Context<'_>,
    ) -> (iced::Task<Message>, Option<Output>) {
        let mut output = None;
        match message {
            Message::ToggleLink => {
                self.link_popover_open = !self.link_popover_open;
                if self.link_popover_open {
                    self.providers = ctx.profiles.account_providers();
                }
            }
            Message::Dismiss => self.link_popover_open = false,
            Message::UnlinkAccount(provider_id) => {
                if !self.mutation_pending && self.link_cancellation.is_none() {
                    let profiles = ctx.profiles.clone();
                    let profile_id = ctx.active_profile.id();
                    self.mutation_pending = true;
                    return (
                        iced::Task::perform(
                            async move {
                                profiles
                                    .unlink_account(profile_id, provider_id)
                                    .await
                                    .map_err(Arc::new)
                            },
                            Message::ProfileUpdated,
                        ),
                        None,
                    );
                }
            }
            Message::BeginLogin(provider) => {
                self.link_popover_open = false;
                if self.link_cancellation.is_none() && !self.mutation_pending {
                    let (cancellation, task) =
                        link_account(ctx.profiles, ctx.active_profile.id(), provider.id);
                    self.link_cancellation = Some(cancellation);
                    self.last_error = None;
                    return (task, None);
                }
            }
            Message::LoginRequested(prompt) => {
                self.login_modal = Some(LoginChallenge {
                    code_draft: String::new(),
                    prompt,
                    submitting: false,
                    error: None,
                });
                output = Some(Output::OpenDialog);
            }
            Message::LoginCodeChanged(code) => {
                if let Some(login) = &mut self.login_modal {
                    login.code_draft = code;
                }
            }
            Message::OpenLoginUrl => {
                if let Some(login) = &self.login_modal {
                    open_url(&login.prompt.url);
                }
            }
            Message::CopyLoginUrl => {
                if let Some(login) = &self.login_modal {
                    return (iced::clipboard::write(login.prompt.url.clone()), None);
                }
            }
            Message::SubmitLoginCode => {
                if let Some(login) = &mut self.login_modal {
                    match login.prompt.submit(login.code_draft.trim()) {
                        Ok(()) => login.submitting = true,
                        Err(error) => login.error = Some(error.to_string()),
                    }
                }
            }
            Message::CancelLogin => {
                self.cancel_active_operation();
                output = Some(Output::CloseDialog);
            }
            Message::ProfileUpdated(result) => output = Some(self.finish_link(result)),
            Message::Noop => {}
        }

        (iced::Task::none(), output)
    }

    fn finish_link(&mut self, result: Result<Profile, Arc<CoreError>>) -> Output {
        self.login_modal = None;
        self.link_cancellation = None;
        self.mutation_pending = false;
        self.last_error = result.err().and_then(|error| {
            (!matches!(error.as_ref(), CoreError::Cancelled)).then(|| error.to_string())
        });
        Output::CloseDialog
    }

    pub fn view_links<'a>(&'a self, ctx: &Context<'a>) -> iced::Element<'a, Message> {
        use iced::widget::{column, container};

        let active = ctx.active_profile;

        let mut accounts = RowGroup::new().title("Linked accounts");
        for account in active.accounts() {
            accounts = accounts.add(account_row(
                provider_icon(&account.provider),
                format!(
                    "{} on {}",
                    account.identity.display_name, account.provider.name
                ),
                "Connected",
                Message::UnlinkAccount(account.provider.id.clone()),
            ));
        }

        let link_trigger = PickerRow::new("Link a storefront account")
            .description("Choose the account provider to connect")
            .on_press(Message::ToggleLink);
        let mut link_popover = Popover::new(link_trigger, self.link_popover_open)
            .on_dismiss(Message::Dismiss)
            .footer("Not listed, install a provider plugin", Message::Noop);

        for provider in &self.providers {
            if active
                .accounts()
                .iter()
                .any(|account| account.provider.id == provider.id)
            {
                continue;
            }

            link_popover = link_popover.add(
                PopoverItem::new(provider.name.as_ref())
                    .icon(provider_icon(provider))
                    .action("Link", Message::BeginLogin(provider.clone())),
            );
        }

        let mut content = column![accounts, container(link_popover).width(iced::Fill)].spacing(18);
        if let Some(error) = &self.last_error {
            content = content.push(crate::widgets::info_card::InfoCard::new(
                crate::widgets::info_card::Kind::Error,
                "Account update failed",
                error,
            ));
        }
        content.into()
    }

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

    let mut content = column![
        Title::new("Sign in").subtitle(&login.prompt.instructions),
        RowGroup::new()
            .add(
                ListRow::from(
                    InfoRow::new("Sign-in link (click to copy)")
                        .description(&login.prompt.url)
                        .icon(Icon::Controller),
                )
                .on_press(Message::CopyLoginUrl),
            )
            .add(action_button_row(
                Icon::Arrow,
                "Open in your browser",
                "Sign in there, then paste the requested value below.",
                "Open",
                Message::OpenLoginUrl,
            ))
            .add(
                TextRow::new("Authorization code", &login.code_draft)
                    .icon(Icon::Checkmark)
                    .on_input(Message::LoginCodeChanged)
                    .on_submit(Message::SubmitLoginCode),
            ),
    ]
    .spacing(18);
    if let Some(error) = &login.error {
        content = content.push(crate::widgets::info_card::InfoCard::new(
            crate::widgets::info_card::Kind::Error,
            "Could not answer the sign-in prompt",
            error,
        ));
    }
    content = content.push(
        row![
            Button::new(submit_label)
                .kind(ButtonKind::Primary)
                .on_press_maybe((!login.submitting).then_some(Message::SubmitLoginCode)),
            Button::new("Cancel")
                .kind(ButtonKind::Transparent)
                .on_press(Message::CancelLogin),
        ]
        .spacing(12),
    );

    container(content)
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

    let labels = column![
        text(title).label().medium(),
        text(description).detail().muted()
    ]
    .spacing(6);

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

fn link_account(
    profiles: &Profiles,
    profile_id: Uuid,
    provider_id: PluginId,
) -> (CancellationToken, iced::Task<Message>) {
    let (send_prompt, prompts) = mpsc::unbounded();
    let interaction = Arc::new(LoginInteraction {
        prompts: send_prompt,
    });
    let operation = profiles.link_account(profile_id, provider_id, interaction);
    let cancellation = operation.cancellation_token();
    let prompts = iced::Task::run(prompts, Message::LoginRequested);
    let operation = iced::Task::perform(operation, |result| {
        Message::ProfileUpdated(result.map_err(Arc::new))
    });
    (cancellation, iced::Task::batch([prompts, operation]))
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

fn provider_icon(provider: &StorefrontProvider) -> Icon {
    if provider.id.as_str() == "steam" {
        Icon::Computer
    } else {
        Icon::Controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_prompt_submission_is_one_shot() {
        futures_lite::future::block_on(async {
            let (prompt, answer) = LoginPrompt::new("https://example.com".into(), "Sign in".into());
            let clone = prompt.clone();

            assert!(prompt.submit("first").is_ok());
            assert_eq!(
                clone.submit("second"),
                Err("account-link prompt was already answered")
            );
            assert_eq!(answer.await.unwrap(), "first");
        });
    }

    #[test]
    fn cancellation_stays_active_until_the_terminal_event() {
        let mut state = State::new();
        let cancellation = CancellationToken::new();
        state.link_cancellation = Some(cancellation.clone());

        state.cancel_active_operation();

        assert!(cancellation.is_cancelled());
        assert!(state.has_active_operation());

        state.finish_link(Err(Arc::new(CoreError::Cancelled)));

        assert!(!state.has_active_operation());
        assert!(state.last_error.is_none());
    }
}
