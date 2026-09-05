use bottles_core::BottleState;
use iced::{
    Element, Length,
    widget::{column, responsive},
};

#[cfg(target_os = "linux")]
use crate::widget::{InfoCard, InfoCardKind};
use crate::{
    Icon,
    features::bottle_settings::{Message, State},
    widget::{ActionRow, CycleRow, ExpanderRow, InfoRow, ListRow, RowGroup, SwitcherRow, spacing},
};

const TRACK_MIN_WIDTH: f32 = 300.0;

pub fn view<'a>(state: &'a State, bottle_state: Option<&'a BottleState>) -> Element<'a, Message> {
    let Some(bottle_state) = bottle_state else {
        return column![].into();
    };
    let content = column![
        responsive(move |size| {
            let columns = usize::from(size.width >= TRACK_MIN_WIDTH * 2.0 + spacing::SM) + 1;
            let bottle_name: ListRow<'_, Message> =
                ListRow::from(InfoRow::new("Bottle Name").description(bottle_state.name()))
                    .trailing(Icon::Pencil.view().width(16).height(16));
            let unavailable = || {
                InfoRow::new("Not available yet").description("This setting is not supported yet.")
            };
            let bottle = RowGroup::new()
                .title("Bottle")
                .columns(columns)
                .row(bottle_name)
                .expander(
                    ExpanderRow::with_header(
                        InfoRow::new("Runner")
                            .description(format!(
                                "{} {}",
                                bottle_state.runner().name(),
                                bottle_state.runner().version()
                            ))
                            .icon(Icon::Run),
                    )
                    .row(unavailable()),
                )
                .expander(
                    ExpanderRow::new("Dependencies")
                        .description("Install fonts, codecs, libraries...")
                        .row(unavailable()),
                )
                .expander(
                    ExpanderRow::new("Drives")
                        .description("Define your custom drives")
                        .row(unavailable()),
                );

            let graphics = RowGroup::new()
                .title("Graphics")
                .columns(columns)
                .row(SwitcherRow::new("DLSS", false).description("Deep Learning Super Sampling"))
                .row(SwitcherRow::new("vkBasalt", false).description("Add post-processing effects"))
                .row(
                    SwitcherRow::new("Discrete GPU", false)
                        .description("Force use your dedicated GPU"),
                )
                .expander(
                    ExpanderRow::with_header(
                        SwitcherRow::new("FSR", false).description("FidelityFX Super Resolution"),
                    )
                    .columns(2)
                    .row(ActionRow::new("Quality", None).description("Balanced"))
                    .row(CycleRow::new("Sharpening", "5")),
                );

            #[cfg(target_os = "linux")]
            let graphics = {
                let wrappers = bottle_state.wrappers();
                graphics.row(
                    SwitcherRow::new("Gamescope", wrappers.gamescope.enabled)
                        .description("Use the SteamOS compositor")
                        .on_toggle(Message::ToggleGamescope),
                )
            };

            let graphics = graphics.expander(
                ExpanderRow::with_header(
                    SwitcherRow::new("Display Settings", false)
                        .description("Resolution and other options"),
                )
                .row(unavailable()),
            );

            #[cfg(target_os = "linux")]
            let graphics = {
                let wrappers = bottle_state.wrappers();
                graphics.row(
                    SwitcherRow::new("MangoHud", wrappers.mangohud.enabled)
                        .description("Show a performance overlay")
                        .on_toggle(Message::ToggleMangoHud),
                )
            };

            column![bottle, graphics].spacing(spacing::SM).into()
        })
        .height(Length::Shrink)
    ];
    #[cfg(target_os = "linux")]
    let content = if let Some(error) = state.last_error() {
        content.push(
            InfoCard::new(
                InfoCardKind::Error,
                "Could not update bottle settings",
                error,
            )
            .width(iced::Fill),
        )
    } else {
        content
    };

    content.into()
}
