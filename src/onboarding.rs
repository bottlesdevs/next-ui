//! First-run experience selection and downloads derived from session snapshots.

use bottles_core::{AddonKind, CatalogEntry, Slot};
use iced::{
    Element, Fill, Length, Task, Theme,
    alignment::{Horizontal, Vertical},
    widget::{column, container, row, scrollable, svg, text},
};

use crate::{
    Experience, Icon, app, chrome,
    context::Context,
    jobs::{Jobs, Kind},
    store::Store,
    theme,
    widget::{
        ActionRow, Button, ButtonKind, InfoCard, InfoCardKind, InfoRow, RowGroup, TextExt as _,
        spacing,
    },
};

const TUTORIALS: [(&str, &str); 3] = [
    (
        "What's a Bottle?",
        "Windows software in {OS} are not installed in the system, those lives in bottles. A bottle is a Windows-like space which makes the software feel in a native Windows environment.",
    ),
    (
        "What are Environments?",
        "When you create a bottle, you must choose an environment, which is a set of configurations and utilities applied to ensure compatibility with the kind of software you want to install, i.e. games, applications.",
    ),
    (
        "First Bottle Matter",
        "The first time you create a bottle using an environment, it takes some minutes. Then the next bottles will takes just a few seconds as the first created will be used as a template for all the others.",
    ),
];

#[derive(Clone, Copy)]
enum Step {
    Welcome,
    Tutorial(usize),
    Downloads,
}

pub struct State {
    step: Step,
    experience: Experience,
}

#[derive(Clone)]
pub enum Message {
    SelectExperience(Experience),
    ApplyExperience,
    NextTutorial,
    Download,
    Cancel,
    Refresh,
    Window(chrome::Action),
    SwitchExperience(Experience),
}

impl State {
    pub fn new() -> Self {
        Self {
            step: Step::Welcome,
            experience: Experience::Classic,
        }
    }

    pub fn update(&mut self, ctx: &mut Context, message: Message) -> Task<app::Message> {
        match message {
            Message::Window(action) => return Task::done(app::Message::Window(action)),
            Message::SwitchExperience(experience) => {
                return Task::done(app::Message::SwitchExperience(experience));
            }
            Message::SelectExperience(experience) => self.experience = experience,
            Message::ApplyExperience => {
                if self.experience == Experience::Next {
                    return self.downloads(ctx);
                }
                self.step = Step::Tutorial(0);
            }
            Message::NextTutorial => {
                if let Step::Tutorial(index) = self.step {
                    if index + 1 < TUTORIALS.len() {
                        self.step = Step::Tutorial(index + 1);
                    } else {
                        return self.downloads(ctx);
                    }
                }
            }
            Message::Refresh => return self.downloads(ctx),
            Message::Download => {
                let entries = essential_entries(&ctx.store);
                let pending = entries
                    .into_iter()
                    .filter(|entry| !installed(&ctx.store, entry))
                    .collect::<Vec<_>>();
                return Task::batch(pending.into_iter().map(|entry| ctx.fetch(entry)));
            }
            Message::Cancel => {
                let entries = essential_entries(&ctx.store);
                for entry in entries {
                    if let Some(job) = ctx.jobs.find(&Kind::Fetch(entry.id())) {
                        job.cancel.cancel();
                    }
                }
            }
        }
        Task::none()
    }

    fn downloads(&mut self, ctx: &mut Context) -> Task<app::Message> {
        self.step = Step::Downloads;
        ctx.jobs
            .start(Kind::RefreshCatalog, ctx.core.addons().refresh())
    }

    pub fn view<'a>(&'a self, ctx: &'a Context) -> Element<'a, Message> {
        let content = match self.step {
            Step::Welcome => self.welcome_view(),
            Step::Tutorial(index) => tutorial_view(index),
            Step::Downloads => self.downloads_view(&ctx.store, &ctx.jobs),
        };
        shell(content)
    }

    fn welcome_view(&self) -> Element<'_, Message> {
        let mut choices = column![].spacing(spacing::SM).width(Length::FillPortion(1));
        for experience in [Experience::Next, Experience::Classic] {
            choices = choices.push(
                ActionRow::new(
                    experience_label(experience),
                    experience
                        .is_available()
                        .then_some(Message::SelectExperience(experience)),
                )
                .description(match experience {
                    Experience::Next => "The easiest way to use Bottles.",
                    Experience::Classic => "The experience for advanced users.",
                })
                .selected(experience == self.experience),
            );
        }
        let detail = match self.experience {
            Experience::Next => "Find and launch your software and games in one library.",
            Experience::Classic => "Manage multiple bottles and fine-tune their settings.",
        };
        column![
            title(
                "Welcome",
                "Choose the experience, you can change this later."
            ),
            row![
                choices,
                InfoCard::new(
                    InfoCardKind::Hint,
                    experience_label(self.experience),
                    detail
                )
                .width(Fill)
                .height(Fill)
            ]
            .spacing(spacing::SM)
            .width(Fill),
            next_button("Apply Experience").on_press(Message::ApplyExperience),
        ]
        .spacing(72)
        .max_width(900)
        .align_x(Horizontal::Center)
        .into()
    }

    fn downloads_view(&self, store: &Store, jobs: &Jobs) -> Element<'_, Message> {
        let preparing = jobs.running(&Kind::RefreshCatalog);
        let entries = essential_entries(store);
        let available = required_available(&entries);
        let mut group = RowGroup::new();
        let mut running = false;
        let complete = !store.addons.runners().is_empty() && !store.addons.winebridges().is_empty();
        for (kind, label) in [
            (AddonKind::Runner, "Runner"),
            (AddonKind::WineBridge, "WineBridge"),
        ] {
            if !entries.iter().any(|entry| entry.kind() == kind) {
                group = group.row(InfoRow::new(label).description(if preparing {
                    "Preparing…"
                } else {
                    "Unavailable for this system. Refresh the catalog to retry."
                }));
            }
        }
        for entry in &entries {
            let label = format!("{} {}", entry.name(), entry.version());
            if installed(store, entry) {
                group = group.row(ActionRow::progress(label, 1.0).description("Installed"));
            } else if let Some(job) = jobs.find(&Kind::Fetch(entry.id())) {
                running = true;
                group = group.row(
                    ActionRow::progress(
                        label,
                        job.progress
                            .as_ref()
                            .and_then(bottles_core::Progress::fraction)
                            .unwrap_or(0.0),
                    )
                    .description(job.progress.as_ref().map_or_else(
                        || "Preparing…".into(),
                        |progress| progress.stage.to_string(),
                    )),
                );
            } else {
                group = group.row(InfoRow::new(label).description("Pending"));
            }
        }
        let action = if complete {
            next_button("Get Started").on_press(Message::SwitchExperience(self.experience))
        } else if preparing {
            button("Preparing…")
        } else if running {
            button("Cancel").on_press(Message::Cancel)
        } else if available {
            next_button("Download").on_press(Message::Download)
        } else {
            button("Refresh catalog").on_press(Message::Refresh)
        };
        column![
            title("Almost Done", "Bottles needs these resources to be ready."),
            container(group).width(Fill).max_width(500),
            action,
        ]
        .spacing(72)
        .align_x(Horizontal::Center)
        .into()
    }
}

const REQUIRED: [AddonKind; 2] = [AddonKind::Runner, AddonKind::WineBridge];
const OPTIONAL: [AddonKind; 4] = [
    AddonKind::Umu,
    AddonKind::Component { slot: Slot::Dxvk },
    AddonKind::Component { slot: Slot::Vkd3d },
    AddonKind::Component {
        slot: Slot::LatencyFlex,
    },
];

fn essential_entries(store: &Store) -> Vec<CatalogEntry> {
    let catalog = store.addons.component_entries();
    REQUIRED
        .into_iter()
        .chain(OPTIONAL)
        .filter_map(|kind| {
            catalog
                .iter()
                .find(|entry| entry.kind() == kind && entry.is_supported())
                .cloned()
        })
        .collect()
}

fn required_available(entries: &[CatalogEntry]) -> bool {
    REQUIRED
        .iter()
        .all(|kind| entries.iter().any(|entry| entry.kind() == *kind))
}

fn installed(store: &Store, entry: &CatalogEntry) -> bool {
    match entry.kind() {
        AddonKind::Runner => store
            .addons
            .runners()
            .iter()
            .any(|addon| addon.id() == entry.id()),
        AddonKind::WineBridge => store
            .addons
            .winebridges()
            .iter()
            .any(|addon| addon.id() == entry.id()),
        AddonKind::Umu => store
            .addons
            .umus()
            .iter()
            .any(|addon| addon.id() == entry.id()),
        AddonKind::Component { .. } => store
            .addons
            .components()
            .iter()
            .any(|addon| addon.id() == entry.id()),
        AddonKind::Dependency => store
            .addons
            .dependencies()
            .iter()
            .any(|addon| addon.id() == entry.id()),
    }
}

pub(crate) fn shell<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let header = chrome::header(Message::Window(chrome::Action::Drag), true, |header| {
        header.transparent(true)
    });
    container(
        column![
            header,
            scrollable(container(content).center(Fill).padding(32))
                .width(Fill)
                .height(Fill)
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::panel)
    .clip(true)
    .into()
}

fn experience_label(experience: Experience) -> &'static str {
    match experience {
        Experience::Next => "Next Mode",
        Experience::Classic => "Classic Mode",
    }
}

fn title<'a>(heading: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    column![
        text(heading)
            .size(32)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            })
            .style(|theme: &Theme| text::Style {
                color: Some(crate::theme::colors(theme).accent)
            }),
        text(subtitle).size(16).medium().muted(),
    ]
    .align_x(Horizontal::Center)
    .spacing(6)
    .into()
}

fn os_label() -> &'static str {
    match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        other => other,
    }
}

fn tutorial_view(index: usize) -> Element<'static, Message> {
    let (heading, body) = TUTORIALS[index];
    let body = body.replace("{OS}", os_label());
    let icon = Icon::Bottles
        .view()
        .width(160)
        .height(160)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(crate::theme::colors(theme).accent),
        });
    column![
        row![
            icon,
            column![title(heading, ""), text(body).body().muted()]
                .spacing(20)
                .max_width(700)
        ]
        .spacing(48)
        .align_y(Vertical::Center),
        next_button("Next").on_press(Message::NextTutorial),
    ]
    .spacing(96)
    .align_x(Horizontal::Center)
    .into()
}

fn button(label: &str) -> Button<'_, Message> {
    Button::custom(text(label).label()).kind(ButtonKind::Primary)
}

fn next_button(label: &str) -> Button<'_, Message> {
    button(label)
        .trailing_icon(Icon::Arrow)
        .icon_rotation(std::f32::consts::PI)
        .icon_size(18.0)
}
