use bottles_core::ProfilesConfig;
use iced::{
    Element,
    widget::{column, container, row},
};

use crate::{
    Icon,
    features::profiles::{self, State},
    widget::{
        Button, ButtonKind, Dialog, InfoCard, InfoCardKind, Popover, PopoverItem, TextRow, Title,
    },
};

pub fn dialog(state: &State) -> Option<Dialog<'_, profiles::Message>> {
    let profile = state.new_profile()?;
    let mut content = column![
        container(Title::new("New profile").subtitle("Give this profile a name."))
            .center_x(iced::Fill),
        TextRow::new("Profile name", profile.name())
            .icon(Icon::Person)
            .on_input(profiles::Message::CreateNameChanged)
            .on_submit(profiles::Message::SubmitCreate),
    ]
    .spacing(18);

    if let Some(error) = profile.error() {
        content = content.push(
            InfoCard::new(InfoCardKind::Error, "Could not create profile", error).width(iced::Fill),
        );
    }

    content = content.push(
        row![
            Button::new("Create")
                .kind(ButtonKind::Primary)
                .on_press(profiles::Message::SubmitCreate)
                .loading(state.is_creating_profile()),
            Button::new("Cancel")
                .kind(ButtonKind::Transparent)
                .on_press(profiles::Message::DismissCreate),
        ]
        .spacing(12),
    );

    Some(Dialog::new(content, profiles::Message::DismissCreate))
}

pub fn switcher(snapshot: &ProfilesConfig) -> Element<'_, super::Message> {
    let selected = snapshot.selected();
    let trigger = Button::icon_only(selected.name(), Icon::Person)
        .diameter(32.0)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent);
    let mut switcher = Popover::new(trigger);

    for profile in snapshot.profiles() {
        switcher = switcher.item(
            PopoverItem::new(profile.name())
                .icon(Icon::Person)
                .selected(selected.id() == profile.id())
                .on_select(super::profiles_message(profiles::Message::ActivateProfile(
                    profile.id(),
                ))),
        );
    }

    switcher
        .item(PopoverItem::new("Profiles").on_select(super::Message::ToggleProfileSettings))
        .into()
}
