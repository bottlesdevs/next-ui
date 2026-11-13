//! The Add Program pane: a program from the computer, added to a bottle in
//! Classic or given a prefix of its own in Next.

use std::path::PathBuf;

use iced::{Element, widget::column};
use next_ui::{
    Icon, chrome,
    layout::{self, PageBody, PaneContext},
    widget::{PickerRow, TextRow, Title, spacing},
};
use uuid::Uuid;

use crate::{
    model::{Action, Model},
    parts,
};

/// Where the program goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A prefix of its own, as Next keeps programs.
    Standalone,
    /// The bottle with this prefix.
    Bottle(Uuid),
}

pub struct State {
    target: Target,
    name: String,
    file: Option<PathBuf>,
    options: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Name(String),
    Options(String),
    PickFile,
    Add,
    Cancel,
    Window(chrome::Action),
}

/// What the screen holding the pane does next.
pub enum Event {
    /// Opens the file picker; its choice comes back through [`State::picked`].
    PickFile,
    Act(Action),
    Close,
    Window(chrome::Action),
}

impl State {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            name: String::new(),
            file: None,
            options: String::new(),
        }
    }

    /// Takes the chosen file, and names the program after it unless a name
    /// was typed.
    pub fn picked(&mut self, file: PathBuf) {
        if self.name.trim().is_empty()
            && let Some(stem) = file.file_stem()
        {
            self.name = stem.to_string_lossy().into_owned();
        }
        self.file = Some(file);
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Name(name) => self.name = name,
            Message::Options(options) => self.options = options,
            Message::PickFile => return Some(Event::PickFile),
            Message::Cancel => return Some(Event::Close),
            Message::Window(action) => return Some(Event::Window(action)),
            Message::Add => {
                let executable = self.file.clone().filter(|_| self.can_add())?;
                let (name, options) = (self.name.trim().to_owned(), self.options.clone());
                return Some(Event::Act(match self.target {
                    Target::Standalone => Action::AddStandalone {
                        name,
                        executable,
                        options,
                    },
                    Target::Bottle(prefix) => Action::AddToBottle {
                        prefix,
                        name,
                        executable,
                        options,
                    },
                }));
            }
        }
        None
    }

    fn can_add(&self) -> bool {
        self.file.is_some() && !self.name.trim().is_empty()
    }

    pub fn view<'a>(&'a self, model: &'a Model, pane: PaneContext) -> Element<'a, Message> {
        let subtitle = match self.target {
            Target::Standalone => "Gets an environment of its own.".to_owned(),
            Target::Bottle(prefix) => model.bottle(prefix).map_or_else(
                || "Adding to a bottle.".to_owned(),
                |bottle| format!("Adding to {}.", bottle.name),
            ),
        };
        let can_add = self.can_add();
        let header = pane.header(Message::Window(chrome::Action::Drag), |header| {
            header
                .start(parts::header_button("Cancel", Icon::Arrow, Message::Cancel))
                .middle(Title::new("Add Program").subtitle(subtitle))
                .end(
                    parts::header_button("Add program", Icon::Checkmark, Message::Add)
                        .on_press_maybe(can_add.then_some(Message::Add)),
                )
        });
        // The folder and the file name say which program it is; the rest of
        // the path would wrap over several lines.
        let file = self.file.as_ref().map_or_else(
            || "Choose the program’s .exe file".to_owned(),
            |file| {
                file.iter()
                    .rev()
                    .take(2)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<PathBuf>()
                    .display()
                    .to_string()
            },
        );
        let mut name = TextRow::new("Name", &self.name)
            .placeholder("Program name")
            .on_input(Message::Name);
        if can_add {
            name = name.on_submit(Message::Add);
        }
        layout::page(
            header,
            PageBody::content(
                column![
                    name,
                    PickerRow::new("File")
                        .description(file)
                        .on_press(Message::PickFile),
                    TextRow::new("Launch Options", &self.options)
                        .placeholder("%command%")
                        .on_input(Message::Options),
                ]
                .spacing(spacing::SM),
            ),
        )
    }
}
