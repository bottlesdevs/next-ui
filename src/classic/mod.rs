//! The complete Classic experience: routes, dialogs, read models, and workflows.
//!
//! A future Next experience is a sibling of this module and cannot access these
//! private workflow modules.

mod accounts_view;
mod bottles_view;
mod layout;
mod library_view;
mod profiles_view;
mod settings_view;
#[cfg(feature = "fvs")]
mod snapshots_view;

use crate::{
    Icon,
    domain::{BottleSnapshot, DomainSnapshots},
    features::{self, bottles, profiles},
    theme,
    widget::{
        ActionRow, Button, ButtonKind, Dialog, InfoCard, InfoCardKind, RowGroup, Tab, Tabs,
        TextRow, Title,
    },
};
use iced::{
    Element, Fill, Task,
    widget::{column, container, keyed_column, scrollable},
};
use layout::{PaneContext, Side, navigation_split, side_panel};
use uuid::Uuid;

const CONTENT_MAX_WIDTH: f32 = 1150.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryTab {
    Bottles,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Programs,
    Settings,
    #[cfg(feature = "fvs")]
    Snapshots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Bottles,
    Bottle { id: Uuid, tab: DetailTab },
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayKind {
    NewBottle,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Overlay {
    kind: OverlayKind,
    open: bool,
}

impl Overlay {
    fn open(kind: OverlayKind) -> Self {
        Self { kind, open: true }
    }

    fn is_open(self, kind: OverlayKind) -> bool {
        self.open && self.kind == kind
    }

    fn close(&mut self) {
        self.open = false;
    }
}

pub struct State {
    route: Route,
    overlay: Option<Overlay>,
}

#[derive(Clone)]
pub enum Message {
    PrimaryTabSelected(PrimaryTab),
    DetailTabSelected(DetailTab),
    BottleSelected(Uuid),
    Back,
    AddBottle,
    CancelBottle,
    ToggleProfileSettings,
    OpenMenu,
    TogglePower,
    DragWindow,
    Feature(features::Message),
}

impl State {
    pub fn new() -> Self {
        Self {
            route: Route::Bottles,
            overlay: None,
        }
    }

    fn primary_tab(&self) -> PrimaryTab {
        match self.route {
            Route::Library => PrimaryTab::Library,
            _ => PrimaryTab::Bottles,
        }
    }

    pub fn resume_after_failed_switch(
        &mut self,
        features: &mut features::State,
        _domain: &DomainSnapshots,
    ) -> Task<features::Message> {
        let library = features.reload_library();
        #[cfg(feature = "fvs")]
        if let Some(snapshot) = self.selected_bottle(_domain) {
            let snapshots = features
                .snapshots
                .load(snapshot.bottle.clone())
                .map(features::Message::Snapshots);
            return Task::batch([library, snapshots]);
        }
        library
    }

    pub fn update(
        &mut self,
        features: &mut features::State,
        message: Message,
        domain: &DomainSnapshots,
    ) -> Task<features::Message> {
        match message {
            Message::PrimaryTabSelected(tab) => {
                self.leave_bottle_route(features);
                self.route = match tab {
                    PrimaryTab::Bottles => Route::Bottles,
                    PrimaryTab::Library => Route::Library,
                };
                if tab == PrimaryTab::Library {
                    return features.reload_library();
                }
            }
            Message::DetailTabSelected(tab) => {
                if let Route::Bottle { id, .. } = self.route {
                    self.route = Route::Bottle { id, tab };
                }
            }
            Message::BottleSelected(id) => {
                if domain.bottle(id).is_none() {
                    return Task::none();
                }
                self.route = Route::Bottle {
                    id,
                    tab: DetailTab::Programs,
                };
                #[cfg(feature = "fvs")]
                features.snapshots.clear();
                #[cfg(feature = "fvs")]
                if let Some(snapshot) = self.selected_bottle(domain) {
                    return features
                        .snapshots
                        .load(snapshot.bottle.clone())
                        .map(features::Message::Snapshots);
                }
            }
            Message::Back => {
                features.profiles.dismiss_dialogs();
                if !self.close_overlay(OverlayKind::Profiles) {
                    self.leave_bottle_route(features);
                    self.route = Route::Bottles;
                }
            }
            Message::AddBottle => {
                features.profiles.dismiss_dialogs();
                self.overlay = Some(Overlay::open(OverlayKind::NewBottle));
                features.bottles.reset_creation();
            }
            Message::CancelBottle => {
                features.bottles.cancel_creation();
                self.close_overlay(OverlayKind::NewBottle);
            }
            Message::ToggleProfileSettings => {
                if self.close_overlay(OverlayKind::Profiles) {
                    features.profiles.dismiss_dialogs();
                } else {
                    self.overlay = Some(Overlay::open(OverlayKind::Profiles));
                }
            }
            Message::OpenMenu | Message::TogglePower | Message::DragWindow => {}
            Message::Feature(_) => unreachable!("feature messages are handled by the app shell"),
        }

        Task::none()
    }

    pub fn bottles_changed(&mut self, features: &mut features::State, domain: &DomainSnapshots) {
        let selected_exists = match self.route {
            Route::Bottle { id, .. } => domain.bottle(id).is_some(),
            Route::Bottles | Route::Library => true,
        };
        if reconcile_route(&mut self.route, selected_exists) {
            self.leave_bottle_route(features);
        }
    }

    pub fn handle_feature_output(&mut self, output: features::Output) {
        match output {
            features::Output::BottleCreated => {
                self.close_overlay(OverlayKind::NewBottle);
            }
        }
    }

    fn leave_bottle_route(&mut self, _features: &mut features::State) {
        #[cfg(feature = "fvs")]
        _features.snapshots.clear();
    }

    fn close_overlay(&mut self, kind: OverlayKind) -> bool {
        let Some(overlay) = self
            .overlay
            .as_mut()
            .filter(|overlay| overlay.is_open(kind))
        else {
            return false;
        };
        overlay.close();
        true
    }

    fn selected_bottle<'a>(&self, domain: &'a DomainSnapshots) -> Option<&'a BottleSnapshot> {
        let Route::Bottle { id, .. } = self.route else {
            return None;
        };
        domain.bottle(id)
    }

    pub fn view<'a>(
        &'a self,
        features: &'a features::State,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Message> {
        let overlay = self.overlay.unwrap_or(Overlay {
            kind: OverlayKind::NewBottle,
            open: false,
        });
        let split = side_panel(
            match overlay.kind {
                OverlayKind::Profiles => Side::End,
                OverlayKind::NewBottle => Side::Start,
            },
            overlay.open,
            |base_context| {
                navigation_split(
                    base_context,
                    matches!(self.route, Route::Bottle { .. }),
                    |context| self.primary_page(context, features, domain),
                    |context| self.detail_page(context, features, domain),
                )
            },
            move |context| match overlay.kind {
                OverlayKind::Profiles => self.profile_settings_page(context, features, domain),
                OverlayKind::NewBottle => self.new_bottle_page(context, features),
            },
        );

        container(split).width(Fill).height(Fill).into()
    }

    pub(crate) fn dialog<'a>(
        &'a self,
        features: &'a features::State,
    ) -> Option<Dialog<'a, Message>> {
        if !self
            .overlay
            .is_some_and(|overlay| overlay.is_open(OverlayKind::Profiles))
        {
            return None;
        }

        profiles_view::dialog(&features.profiles)
            .map(|dialog| dialog.map(profiles_message))
            .or_else(|| {
                accounts_view::dialog(&features.profiles).map(|dialog| dialog.map(profiles_message))
            })
    }

    fn primary_page<'a>(
        &'a self,
        context: PaneContext,
        features: &'a features::State,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(PrimaryTab::Bottles, "Bottles"),
                Tab::new(PrimaryTab::Library, "Library"),
            ],
            Some(self.primary_tab()),
            Message::PrimaryTabSelected,
        );
        let header = context.header(Message::DragWindow, |header| {
            header
                .start(header_button("Add bottle", Icon::Plus, Message::AddBottle))
                .middle(tabs)
                .end(profiles_view::switcher(domain.profiles()))
        });
        let content: Element<'_, Message> = match self.primary_tab() {
            PrimaryTab::Bottles => bottles_view::rows(
                domain.bottles(),
                match self.route {
                    Route::Bottle { id, .. } => Some(id),
                    _ => None,
                },
                Message::BottleSelected,
            ),
            PrimaryTab::Library => library_view::view(&features.library).map(library_message),
        };

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn profile_settings_page<'a>(
        &'a self,
        context: PaneContext,
        features: &'a features::State,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Message> {
        let header = context.header(Message::DragWindow, |header| {
            header
                .start(header_button("Cancel", Icon::Arrow, Message::Back))
                .middle(
                    container(
                        Title::new("Profile Settings")
                            .subtitle("Manage your profiles and linked accounts."),
                    )
                    .padding(iced::padding::bottom(12)),
                )
                .end(header_button(
                    "New profile",
                    Icon::Plus,
                    profiles_message(profiles::Message::OpenCreate),
                ))
        });

        let content: Element<'_, Message> = {
            let active = domain.profiles().selected();
            let links = accounts_view::links(&features.profiles, active).map(profiles_message);

            column![
                RowGroup::new()
                    .title("Experience")
                    .row(
                        ActionRow::new("Classic", None)
                            .description("Current experience")
                            .icon(Icon::Checkmark),
                    )
                    .row(
                        ActionRow::new("Next", None)
                            .description("Not available yet")
                            .icon(Icon::Wand),
                    ),
                RowGroup::new()
                    .title("Profile")
                    .row(
                        TextRow::new("Profile name", features.profiles.name_draft())
                            .icon(Icon::Person)
                            .on_input(|name| profiles_message(profiles::Message::NameChanged(name)))
                            .on_submit(profiles_message(profiles::Message::RenameSubmit)),
                    )
                    .row(accounts_view::action_button_row(
                        Icon::Cross,
                        "Delete profile",
                        "Removes this profile and its linked accounts from this device",
                        "Delete",
                        profiles_message(profiles::Message::DeleteProfile(active.id())),
                    )),
                links,
            ]
            .spacing(18)
            .into()
        };
        let content: Element<'_, Message> = if let Some(error) = features.profiles.last_error() {
            column![
                InfoCard::new(InfoCardKind::Error, "Could not update profile", error,).width(Fill),
                content,
            ]
            .spacing(18)
            .into()
        } else {
            content
        };

        let page: Element<'_, Message> = column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into();

        page
    }

    fn new_bottle_page<'a>(
        &'a self,
        context: PaneContext,
        features: &'a features::State,
    ) -> Element<'a, Message> {
        let header = context.header(Message::DragWindow, |header| {
            header
                .start(header_button(
                    "Cancel bottle creation",
                    Icon::Arrow,
                    Message::CancelBottle,
                ))
                .middle(
                    container(Title::new("New Bottle").subtitle("Creating a new bottle."))
                        .padding(iced::padding::bottom(12)),
                )
                .end(
                    header_button(
                        "Create bottle",
                        Icon::Checkmark,
                        bottles_message(bottles::Message::CreateBottle),
                    )
                    .on_press_maybe(
                        (!features.bottles.is_creating())
                            .then_some(bottles_message(bottles::Message::CreateBottle)),
                    ),
                )
        });
        let content = bottles_view::creation(&features.bottles).map(bottles_message);

        column![header, scroll_panel(content)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn detail_page<'a>(
        &'a self,
        context: PaneContext,
        features: &'a features::State,
        domain: &'a DomainSnapshots,
    ) -> Element<'a, Message> {
        #[cfg(feature = "fvs")]
        let detail_tabs = [
            Tab::new(DetailTab::Programs, "Programs"),
            Tab::new(DetailTab::Settings, "Settings"),
            Tab::new(DetailTab::Snapshots, "Snapshots"),
        ];
        #[cfg(not(feature = "fvs"))]
        let detail_tabs = [
            Tab::new(DetailTab::Programs, "Programs"),
            Tab::new(DetailTab::Settings, "Settings"),
        ];
        let tabs = Tabs::new(
            detail_tabs,
            Some(match self.route {
                Route::Bottle { tab, .. } => tab,
                _ => DetailTab::Programs,
            }),
            Message::DetailTabSelected,
        );
        let header = context.header(Message::DragWindow, |mut header| {
            if context.is_standalone() {
                header = header.start(
                    Button::icon_only("Back to bottles", Icon::Arrow)
                        .diameter(32.0)
                        .icon_size(16.0)
                        .kind(ButtonKind::Transparent)
                        .on_press(Message::Back),
                );
            }

            header
                .start(header_button(
                    "More actions",
                    Icon::EllipsisVertical,
                    Message::OpenMenu,
                ))
                .start(header_button(
                    "Toggle power",
                    Icon::Power,
                    Message::TogglePower,
                ))
                .middle(tabs)
        });
        let detail_tab = match self.route {
            Route::Bottle { tab, .. } => tab,
            _ => DetailTab::Programs,
        };
        let content = match detail_tab {
            DetailTab::Programs => {
                if let Some(snapshot) = self.selected_bottle(domain) {
                    bottles_view::programs(
                        &features.bottles,
                        snapshot.bottle.clone(),
                        snapshot.state.as_ref(),
                    )
                    .map(bottles_message)
                } else {
                    column![].into()
                }
            }
            DetailTab::Settings => {
                if let Some(snapshot) = self.selected_bottle(domain) {
                    let bottle_id = snapshot.state.id();
                    settings_view::view(&features.settings, bottle_id, snapshot.state.as_ref())
                        .map(settings_message)
                } else {
                    column![].into()
                }
            }
            #[cfg(feature = "fvs")]
            DetailTab::Snapshots => {
                snapshots_view::view(&features.snapshots).map(snapshots_message)
            }
        };

        let mut body = column![scroll_content(content)].width(Fill).height(Fill);

        if let Some(snapshot) = self.selected_bottle(domain) {
            let state = snapshot.state.as_ref();
            body = body.push(
                keyed_column([(state.id(), bottles_view::status(state).map(bottles_message))])
                    .width(Fill),
            );
        }

        let body = container(body)
            .width(Fill)
            .height(Fill)
            .style(theme::panel)
            .clip(true);

        column![header, body].width(Fill).height(Fill).into()
    }
}

fn reconcile_route(route: &mut Route, selected_exists: bool) -> bool {
    if matches!(route, Route::Bottle { .. }) && !selected_exists {
        *route = Route::Bottles;
        true
    } else {
        false
    }
}

fn scroll_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(scroll_content(content))
        .width(Fill)
        .height(Fill)
        .style(theme::panel)
        .clip(true)
        .into()
}

fn scroll_content<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let content = container(content).width(Fill).max_width(CONTENT_MAX_WIDTH);
    let content = container(content).padding(24).center_x(Fill);

    container(
        scrollable(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4)
                    .margin(12),
            ))
            .style(theme::scrollbar)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

fn header_button(label: &str, icon: Icon, message: Message) -> Button<'_, Message> {
    Button::icon_only(label, icon)
        .diameter(32.0)
        .icon_size(16.0)
        .kind(ButtonKind::Transparent)
        .on_press(message)
}

fn bottles_message(message: bottles::Message) -> Message {
    Message::Feature(features::Message::Bottles(message))
}

fn profiles_message(message: profiles::Message) -> Message {
    Message::Feature(features::Message::Profiles(message))
}

fn library_message(message: features::library::Message) -> Message {
    Message::Feature(features::Message::Library(message))
}

fn settings_message(message: features::bottle_settings::Message) -> Message {
    Message::Feature(features::Message::Settings(message))
}

#[cfg(feature = "fvs")]
fn snapshots_message(message: features::snapshots::Message) -> Message {
    Message::Feature(features::Message::Snapshots(message))
}
