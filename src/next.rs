//! The Next experience: one library, with details for standalone programs.

use bottles_core::Slot;
use iced::{Element, Fill, widget::text};
use next_ui::{
    Icon, chrome,
    layout::{self, PaneContext, Side},
    widget::{InfoRow, RowGroup, Search, SwitcherRow, TextExt as _, spacing},
};
use uuid::Uuid;

use crate::{
    app::Request,
    manage_profiles,
    model::{Action, Activity, Experience, Model, Program, Subject},
    parts,
};

pub struct State {
    query: String,
    selected: Option<Uuid>,
    profiles: manage_profiles::State,
}

#[derive(Clone)]
pub enum Message {
    QueryChanged(String),
    Select(Uuid),
    Back,
    Act(Action),
    Window(chrome::Action),
    Profiles(manage_profiles::Message),
    OpenPreferences,
}

impl State {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected: None,
            profiles: manage_profiles::State::default(),
        }
    }

    pub fn update(&mut self, model: &Model, message: Message) -> Option<Request> {
        match message {
            Message::Profiles(message) => return self.profiles.update(model, message),
            Message::Window(action) => return Some(Request::Window(action)),
            Message::OpenPreferences => return Some(Request::OpenPreferences),
            Message::Act(action) => return Some(Request::Act(action)),
            Message::QueryChanged(query) => self.query = query,
            Message::Select(id) => self.selected = Some(id),
            Message::Back => self.selected = None,
        }
        None
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let selected = self.selected(model).is_some();
        layout::side_panel(
            Side::End,
            self.profiles.is_open(),
            move |pane| {
                layout::navigation_split(
                    pane,
                    selected,
                    move |pane| self.programs_view(model, pane, selected),
                    move |pane| self.detail_view(model, pane),
                )
            },
            move |pane| self.profiles.pane(model, pane).map(Message::Profiles),
        )
    }

    fn programs_view<'a>(
        &'a self,
        model: &'a Model,
        pane: PaneContext,
        selected: bool,
    ) -> Element<'a, Message> {
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = header.start(parts::app_menu(
                Experience::Next,
                Message::OpenPreferences,
                |experience| Message::Act(Action::SwitchExperience(experience)),
            ));
            let header = if selected {
                header.start(parts::header_button(
                    "Back to library",
                    Icon::Arrow,
                    Message::Back,
                ))
            } else {
                header
            };
            let header = parts::app_controls(header, model, pane, Message::Profiles, Message::Act);
            header.middle(
                iced::widget::container(Search::new(
                    "Search library",
                    &self.query,
                    Message::QueryChanged,
                ))
                .width(Fill)
                .max_width(550),
            )
        });
        let body = parts::library_grid(
            model,
            &self.query,
            |target| Message::Act(Action::Launch(target)),
            |target| target.standalone().map(Message::Select),
            Message::Act(Action::AddProgram),
            |name| Message::Act(Action::InstallSuggestion(name)),
            true,
        );
        parts::body_page(header, body)
    }

    fn detail_view<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let Some(program) = self.selected(model) else {
            return parts::page(
                pane.header(Message::Window(chrome::Action::Drag), |header| header),
                text("No program selected"),
            );
        };
        let stop = Message::Act(Action::Stop(program.prefix));
        let running = model.activity(&program.target().subject()) == Activity::Running;
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            let header = if pane.is_standalone() {
                header.start(parts::header_button(
                    "Back to library",
                    Icon::Arrow,
                    Message::Back,
                ))
            } else {
                header
            };
            let header = header.middle(text(program.name)).end(
                parts::header_button("Stop program", Icon::Power, stop.clone())
                    .on_press_maybe(running.then_some(stop)),
            );
            parts::app_controls(header, model, pane, Message::Profiles, Message::Act)
        });
        let editable = model
            .activity(&Subject::Prefix(program.prefix))
            .is_settled();
        let switch = |slot, title, description| {
            component_switch(model, &program, slot, title, description, editable)
        };
        parts::page(
            header,
            iced::widget::column![
                RowGroup::new()
                    .row(InfoRow::new("Executable").description(program.spec.executable()))
                    .row(InfoRow::new("Runner").description(program.state.runner.name())),
                RowGroup::new()
                    .title("Graphics")
                    .row(switch(Slot::Dxvk, "DXVK", "Vulkan-based D3D9/D3D10/D3D11"))
                    .row(switch(Slot::Vkd3d, "VKD3D-Proton", "Vulkan-based D3D12"))
                    .row(switch(
                        Slot::LatencyFlex,
                        "LatencyFlex",
                        "Lower input latency"
                    )),
                text(
                    "Next always uses the newest downloaded release. Turning a component on \
                     downloads it if needed."
                )
                .size(14)
                .muted(),
            ]
            .spacing(spacing::SM),
        )
    }

    fn selected<'a>(&self, model: &'a Model) -> Option<Program<'a>> {
        model.program(self.selected?)
    }
}

/// A switch for `slot` in the program's prefix. Turning it on selects the
/// newest downloaded release, or downloads the newest one and then selects it.
fn component_switch<'a>(
    model: &'a Model,
    program: &Program<'a>,
    slot: Slot,
    title: &'a str,
    description: &'a str,
    editable: bool,
) -> SwitcherRow<'a, Message> {
    let prefix = program.prefix;
    let on = program.state.component(slot).is_some();
    let downloaded = model.component_releases(slot).first().map(|(id, _)| *id);
    let newest = model.available_components(slot).into_iter().next();
    let downloading = newest.as_ref().and_then(|release| {
        let download = release.download.as_ref()?;
        (download.then_select == Some(prefix)).then(|| {
            let percent = download
                .progress
                .map(|progress| format!(" {}%", (progress * 100.0).round()))
                .unwrap_or_default();
            format!("Downloading {}{percent}", release.label)
        })
    });
    let action = if on {
        Some(Action::RemoveComponent { prefix, slot })
    } else if let Some(release) = downloaded {
        Some(Action::UseRelease { prefix, release })
    } else if downloading.is_none() {
        newest.map(|release| Action::FetchRelease {
            prefix,
            release: release.id,
        })
    } else {
        None
    };
    let row = SwitcherRow::new(title, on || downloading.is_some());
    let row = match (downloading, &action) {
        (Some(status), _) => row.description(status),
        (None, None) => row.description("Not available on this system"),
        (None, Some(_)) => row.description(description),
    };
    row.on_toggle_maybe(
        action
            .filter(|_| editable)
            .map(|action| move |_| Message::Act(action.clone())),
    )
}
