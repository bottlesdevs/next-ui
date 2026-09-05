use bottles_core::{Profile, StorefrontProvider};
use iced::{
    Element,
    widget::{column, container, row},
};

use crate::{
    Icon,
    features::profiles::{self, AccountMessage, LoginDialog, State},
    widget::{
        ActionRow, Button, ButtonKind, Dialog, InfoCard, InfoCardKind, InfoRow, ListRow, PickerRow,
        Popover, PopoverItem, RowGroup, TextRow, Title,
    },
};

pub fn dialog(state: &State) -> Option<Dialog<'_, profiles::Message>> {
    state.account_link().dialog().map(|dialog| {
        Dialog::new(login_dialog(dialog), AccountMessage::DismissLogin)
            .map(profiles::Message::Account)
    })
}

pub fn links<'a>(state: &'a State, active: &'a Profile) -> Element<'a, profiles::Message> {
    let mut accounts = RowGroup::new().title("Linked accounts");
    for account in active.accounts() {
        accounts = accounts.row(action_button_row(
            provider_icon(&account.provider),
            format!(
                "{} on {}",
                account.identity.display_name, account.provider.name
            ),
            "Connected",
            "Unlink",
            profiles::Message::Account(AccountMessage::UnlinkAccount(account.provider.id.clone())),
        ));
    }

    let trigger = PickerRow::new("Link a storefront account")
        .description("Choose the account provider to connect");
    let mut menu = Popover::row(trigger);

    for provider in state.manager().account_providers() {
        if active
            .accounts()
            .iter()
            .any(|account| account.provider.id == provider.id)
        {
            continue;
        }

        menu = menu.item(
            PopoverItem::new(provider.name.clone())
                .icon(provider_icon(&provider))
                .action(
                    "Link",
                    profiles::Message::Account(AccountMessage::BeginLogin(provider)),
                ),
        );
    }

    menu = menu.item(
        PopoverItem::new("Not listed, install a provider plugin").on_select(
            profiles::Message::Account(AccountMessage::InstallProviderPlugin),
        ),
    );

    let mut content = column![accounts, container(menu).width(iced::Fill)].spacing(18);
    if let Some(error) = state.account_link().last_error() {
        content = content.push(
            InfoCard::new(InfoCardKind::Error, "Account update failed", error).width(iced::Fill),
        );
    }

    content.into()
}

pub fn action_button_row<'a, Message: Clone + 'a>(
    icon: Icon,
    title: impl iced::widget::text::IntoFragment<'a>,
    description: &'a str,
    button_label: &'a str,
    on_press: Message,
) -> ListRow<'a, Message> {
    ListRow::from(InfoRow::new(title).description(description).icon(icon)).trailing(
        Button::new(button_label)
            .kind(ButtonKind::Surface)
            .on_press(on_press),
    )
}

fn login_dialog(dialog: &LoginDialog) -> iced::widget::Column<'_, AccountMessage> {
    let submit_label = if dialog.is_submitting() {
        "Submitting…"
    } else {
        "Submit"
    };

    let mut content = column![
        container(Title::new("Sign in").subtitle(dialog.instructions())).center_x(iced::Fill),
        RowGroup::new()
            .row(
                ActionRow::new("Sign-in link (click to copy)", AccountMessage::CopyLoginUrl,)
                    .description(dialog.url())
                    .icon(Icon::Controller),
            )
            .row(action_button_row(
                Icon::Arrow,
                "Open in your browser",
                "Sign in there, then paste the requested value below.",
                "Open",
                AccountMessage::OpenLoginUrl,
            ))
            .row(
                TextRow::new("Authorization code", dialog.code())
                    .icon(Icon::Checkmark)
                    .on_input(AccountMessage::LoginCodeChanged)
                    .on_submit(AccountMessage::SubmitLogin),
            ),
    ]
    .spacing(18);

    if let Some(error) = dialog.error() {
        content = content.push(
            InfoCard::new(
                InfoCardKind::Error,
                "Could not answer the sign-in prompt",
                error,
            )
            .width(iced::Fill),
        );
    }

    content.push(
        row![
            Button::new(submit_label)
                .kind(ButtonKind::Primary)
                .on_press_maybe((!dialog.is_submitting()).then_some(AccountMessage::SubmitLogin)),
            Button::new("Cancel")
                .kind(ButtonKind::Transparent)
                .on_press(AccountMessage::DismissLogin),
        ]
        .spacing(12),
    )
}

fn provider_icon(provider: &StorefrontProvider) -> Icon {
    if provider.id.as_str() == "steam" {
        Icon::Computer
    } else {
        Icon::Controller
    }
}
