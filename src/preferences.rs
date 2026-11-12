//! Preferences: a full-window page with General, Addons, Plugins and About
//! tabs.

use iced::{
    Alignment, Element, Fill,
    widget::{column, container, row, text},
};
use next_ui::{
    Icon, chrome,
    layout::{PaneContext, Side, side_panel},
    widget::{
        ActionRow, Avatar, AvatarShape, BoxedGroup, Button, ButtonKind, FilterChips, ListRow,
        Popover, RowGroup, Search, StatusPage, Tab, Tabs, Tag, TextExt as _, Title, spacing,
    },
};

use crate::{
    app::Request,
    model::{Action, Experience, InstalledPlugin, Listing, Model},
    parts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    General,
    Addons,
    Plugins,
    About,
}

#[derive(Default)]
pub struct State {
    page: Page,
    /// The plugin whose details are open beside the Plugins page.
    plugin: Option<String>,
    /// What the Plugins page is narrowed to.
    plugin_query: String,
    plugin_kind: PluginKind,
}

/// Which plugins the Plugins page lists, by what they provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluginKind {
    #[default]
    All,
    Accounts,
    Libraries,
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(Page),
    Back,
    Act(Action),
    /// Closes a confirmation without doing anything.
    Dismiss,
    OpenPlugin(String),
    ClosePlugin,
    PluginQuery(String),
    PluginKind(PluginKind),
    Window(chrome::Action),
}

impl State {
    /// Opens Preferences at `page`.
    pub fn at(page: Page) -> Self {
        Self {
            page,
            ..Self::default()
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Page(page) => {
                self.page = page;
                self.plugin = None;
            }
            Message::Back => return Some(Request::ClosePreferences),
            Message::Act(action) => {
                if matches!(&action, Action::UninstallPlugin(id) if self.plugin.as_ref() == Some(id))
                {
                    self.plugin = None;
                }
                return Some(Request::Act(action));
            }
            Message::OpenPlugin(id) => self.plugin = Some(id),
            Message::ClosePlugin => self.plugin = None,
            Message::PluginQuery(query) => self.plugin_query = query,
            Message::PluginKind(kind) => self.plugin_kind = kind,
            Message::Dismiss => {}
            Message::Window(action) => return Some(Request::Window(action)),
        }
        None
    }

    pub fn view<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        side_panel(
            Side::End,
            self.plugin.is_some(),
            move |pane| self.main(model, pane),
            move |pane| self.plugin_pane(model, pane),
        )
    }

    fn main<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let tabs = Tabs::new(
            [
                Tab::new(Page::General, "General"),
                Tab::new(Page::Addons, "Addons"),
                Tab::new(Page::Plugins, "Plugins"),
                Tab::new(Page::About, "About"),
            ],
            Some(self.page),
            Message::Page,
        )
        .height(Fill);
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button("Back", Icon::Arrow, Message::Back))
                .middle(tabs)
        });
        let body = match self.page {
            Page::General => Ok(general(model)),
            Page::Addons => addons(model),
            Page::Plugins => plugins(model, self),
            Page::About => Ok(about()),
        };
        match body {
            Ok(body) => parts::page(
                header,
                container(container(body).max_width(880)).center_x(Fill),
            ),
            Err(empty) => parts::status_page(header, empty),
        }
    }

    /// The open plugin: what it adds to Bottles, and Uninstall.
    fn plugin_pane<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button(
                    "Close plugin",
                    Icon::Arrow,
                    Message::ClosePlugin,
                ))
                .middle(Title::new("Plugin"))
        });
        let plugin = self.plugin.as_ref().and_then(|id| {
            model
                .installed_plugins()
                .into_iter()
                .find(|plugin| &plugin.id == id)
        });
        let Some(plugin) = plugin else {
            return parts::page(header, column![]);
        };
        // An update banner, the description, the author and the size of the
        // plugin's data have no source yet (`Model::plugin_update`,
        // `Model::plugin_about`, `Model::plugin_storage`).
        let hero = column![
            Avatar::new(initial(&plugin.name))
                .size(64.0)
                .shape(AvatarShape::Rounded),
            text(plugin.name.clone()).size(24).medium(),
            text(plugin.version.clone()).size(15).muted(),
            tags(&plugin),
        ]
        .spacing(spacing::SM)
        .align_x(Alignment::Center);

        let mut adds = RowGroup::new().title("What it adds");
        if plugin.account {
            adds = adds.row(
                ActionRow::new(format!("{} account", plugin.name), None)
                    .description("Link it in Manage Profiles")
                    .leading(icon_tile(Icon::People)),
            );
        }
        if plugin.library {
            let summary = match model.sources.listings.get(&plugin.id) {
                Some(Listing::Loaded(entries)) => match entries.len() {
                    1 => "1 title in your library".to_owned(),
                    count => format!("{count} titles in your library"),
                },
                Some(Listing::Failed(error)) => error.clone(),
                Some(Listing::Loading) | None => "Loading…".to_owned(),
            };
            adds = adds.row(
                ActionRow::new(format!("{} library", plugin.name), None)
                    .description(summary)
                    .leading(icon_tile(Icon::Library)),
            );
        }

        let uninstall = Popover::confirm(
            Button::new(format!("Uninstall {}", plugin.name))
                .icon(Icon::Trash)
                .kind(ButtonKind::Destructive),
            format!("Uninstall {}?", plugin.name),
            "You can install it again later.",
            "Uninstall",
            Message::Act(Action::UninstallPlugin(plugin.id.clone())),
            Message::Dismiss,
        );
        parts::page(
            header,
            column![
                container(hero).center_x(Fill).padding([spacing::MD, 0.0]),
                adds,
                container(uninstall).center_x(Fill),
            ]
            .spacing(spacing::LG),
        )
    }
}

fn initial(name: &str) -> String {
    name.chars().take(1).collect()
}

/// Tags for what `plugin` provides.
fn tags<'a>(plugin: &InstalledPlugin) -> Element<'a, Message> {
    let mut tags = row![].spacing(6);
    if plugin.account {
        tags = tags.push(Tag::new("Account"));
    }
    if plugin.library {
        tags = tags.push(Tag::new("Library"));
    }
    tags.into()
}

fn general(model: &Model) -> Element<'_, Message> {
    let current = model.settings.experience;
    let choice = |experience: Experience, label, description| {
        let available = experience.is_available();
        let selected = current == Some(experience);
        let mut row = ActionRow::new(
            label,
            available.then_some(Message::Act(Action::SwitchExperience(experience))),
        )
        .description(if available {
            description
        } else {
            "Not available in this build"
        })
        .selected(selected);
        if selected {
            row = row.trailing(Icon::Checkmark.view());
        }
        row
    };

    RowGroup::new()
        .title("Experience")
        .columns(2)
        .row(choice(
            Experience::Next,
            "Next Mode",
            "The easiest way to use Bottles.",
        ))
        .row(choice(
            Experience::Classic,
            "Classic Mode",
            "The experience for advanced users.",
        ))
        .into()
}

/// A tab's scrolling content, or the status page shown when it has none.
type Body<'a> = Result<Element<'a, Message>, Element<'a, Message>>;

fn addons(model: &Model) -> Body<'_> {
    let groups = model.release_groups();
    if groups.is_empty() {
        return Err(StatusPage::new("Nothing downloaded")
            .description("Runners and components you download appear here.")
            .into());
    }
    // Whether a bottle uses a release isn't known yet (`Model::release_users`),
    // so every release offers Remove.
    Ok(groups
        .into_iter()
        .fold(column![].spacing(spacing::LG), |page, (title, releases)| {
            let icon = match title {
                "Runners" => Icon::Run,
                "WineBridge and UMU" => Icon::Computer,
                "Dependencies" => Icon::Library,
                _ => Icon::Chip,
            };
            page.push(releases.into_iter().fold(
                BoxedGroup::new().title(title).icon(icon),
                |group, (id, label)| {
                    group.row(ListRow::new(text(label).label().medium()).trailing(
                        Popover::confirm(
                            Button::new("Remove").icon(Icon::Trash),
                            "Remove this release?",
                            "You can download it again later.",
                            "Remove",
                            Message::Act(Action::RemoveRelease(id)),
                            Message::Dismiss,
                        ),
                    ))
                },
            ))
        })
        .into())
}

fn plugins<'a>(model: &'a Model, state: &'a State) -> Body<'a> {
    let installed = model.installed_plugins();
    // The catalog of available plugins doesn't exist yet, and the manifests
    // have no description, author or update (`Model::plugin_about`,
    // `Model::plugin_update`), so the page lists installed plugins by name.
    if installed.is_empty() {
        return Err(StatusPage::new("No plugins installed")
            .description("Plugins add storefront accounts and libraries.")
            .into());
    }
    let query = state.plugin_query.to_lowercase();
    let shown = installed
        .into_iter()
        .filter(|plugin| match state.plugin_kind {
            PluginKind::All => true,
            PluginKind::Accounts => plugin.account,
            PluginKind::Libraries => plugin.library,
        })
        .filter(|plugin| plugin.name.to_lowercase().contains(&query))
        .collect::<Vec<_>>();

    // Installing from a file needs the plugin host to read a package
    // without installing it (next-plugin-host#6).
    let toolbar = row![
        Search::new("Search plugins…", &state.plugin_query, Message::PluginQuery),
        Button::new("From File").icon(Icon::Folder),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    let chips = FilterChips::new(
        [
            (PluginKind::All, "All"),
            (PluginKind::Accounts, "Accounts"),
            (PluginKind::Libraries, "Libraries"),
        ],
        state.plugin_kind,
    )
    .on_select(Message::PluginKind);

    let open = state.plugin.as_deref();
    let list: Element<'a, Message> = if shown.is_empty() {
        StatusPage::new("No plugins found")
            .description("No installed plugin matches.")
            .into()
    } else {
        let heading = text(format!("Installed · {}", shown.len()))
            .size(15)
            .medium()
            .style(|theme| text::Style {
                color: Some(next_ui::theme::colors(theme).accent_muted),
            });
        let group = shown.into_iter().fold(BoxedGroup::new(), |group, plugin| {
            group.row(
                ActionRow::new(plugin.name.clone(), Message::OpenPlugin(plugin.id.clone()))
                    .title_suffix(text(plugin.version.clone()).size(13).muted())
                    .title_suffix(tags(&plugin))
                    .leading(Avatar::new(initial(&plugin.name)).shape(AvatarShape::Rounded))
                    .selected(open == Some(plugin.id.as_str()))
                    .trailing(Popover::confirm(
                        Button::new("Uninstall").icon(Icon::Trash),
                        format!("Uninstall {}?", plugin.name),
                        "You can install it again later.",
                        "Uninstall",
                        Message::Act(Action::UninstallPlugin(plugin.id)),
                        Message::Dismiss,
                    ))
                    .trailing(Icon::ChevronRight.view()),
            )
        });
        column![heading, group].spacing(spacing::SM).into()
    };
    Ok(column![toolbar, chips, list].spacing(spacing::MD).into())
}

fn about<'a>() -> Element<'a, Message> {
    let hero = column![
        Avatar::new("B")
            .size(104.0)
            .shape(AvatarShape::Rounded)
            .active(true),
        text("Bottles Next").size(30).medium(),
        text("Run Windows software and games, each in a place of its own.")
            .size(17)
            .muted(),
        text(concat!("Version ", env!("CARGO_PKG_VERSION")))
            .size(14)
            .muted(),
    ]
    .spacing(spacing::SM)
    .align_x(Alignment::Center);

    // Opening links needs a URL opener, which the app doesn't have yet.
    let link = |title, description, icon| {
        ActionRow::new(title, None)
            .description(description)
            .leading(icon_tile(icon))
            .trailing(Icon::ExternalLink.view())
    };
    let page = |title, description, icon| {
        ActionRow::new(title, None)
            .description(description)
            .leading(icon_tile(icon))
            .trailing(Icon::ChevronRight.view())
    };

    let content = column![
        container(hero).center_x(Fill).padding([spacing::LG, 0.0]),
        RowGroup::new()
            .row(link(
                "Report a Problem",
                "Opens the issue tracker",
                Icon::Bug
            ))
            .row(link(
                "Source Code",
                "Bottles is free software. Read it, fork it, help out.",
                Icon::Code,
            )),
        RowGroup::new()
            .row(page(
                "Credits",
                "The people who make and translate Bottles",
                Icon::People,
            ))
            .row(page(
                "Legal",
                "License and third-party notices",
                Icon::Document
            )),
    ]
    .spacing(spacing::MD)
    .max_width(640);

    container(content).center_x(Fill).into()
}

fn icon_tile<'a>(icon: Icon) -> Element<'a, Message> {
    container(icon.view().width(22).height(22))
        .center(40)
        .style(|theme| {
            let colors = next_ui::theme::colors(theme);
            container::Style::default()
                .background(colors.border)
                .border(iced::Border::default().rounded(10))
        })
        .into()
}
