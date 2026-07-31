use iced::{ContentFit, Element, widget::svg};
use rust_embed::RustEmbed;

pub const SIZE: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Arrow,
    Bottles,
    Checkmark,
    Chip,
    Computer,
    Controller,
    Cross,
    Custom,
    Disk,
    DoubleCheckmark,
    DownCaret,
    EllipsisVertical,
    Error,
    Folder,
    Gear,
    HollowGear,
    Info,
    Lightning,
    Pencil,
    Person,
    Play,
    Plus,
    Power,
    Run,
    Search,
    Stop,
    Timer,
    Wand,
    Warning,
}

impl Icon {
    pub fn handle(self) -> svg::Handle {
        let path = format!("icons/{}.svg", self.name());
        let icon = Assets::get(&path)
            .unwrap_or_else(|| unreachable!("typed embedded icon is missing: {path}"));

        svg::Handle::from_memory(icon.data)
    }

    pub fn view<'a, Message: 'a>(self) -> Element<'a, Message> {
        self.rotated(0.0)
    }

    pub fn rotated<'a, Message: 'a>(self, rotation: f32) -> Element<'a, Message> {
        svg(self.handle())
            .width(SIZE)
            .height(SIZE)
            .content_fit(ContentFit::Contain)
            .rotation(rotation)
            .into()
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Bottles => "bottles-symbolic",
            Self::Checkmark => "checkmark",
            Self::Chip => "chip",
            Self::Computer => "computer",
            Self::Controller => "controller",
            Self::Cross => "cross",
            Self::Custom => "custom",
            Self::Disk => "disk",
            Self::DoubleCheckmark => "double_checkmark",
            Self::DownCaret => "down_caret",
            Self::EllipsisVertical => "ellipsis_vertical",
            Self::Error => "error",
            Self::Folder => "folder",
            Self::Gear => "gear",
            Self::HollowGear => "hollow-gear",
            Self::Info => "info",
            Self::Lightning => "lightning",
            Self::Pencil => "pencil",
            Self::Person => "person",
            Self::Play => "play",
            Self::Plus => "plus",
            Self::Power => "power",
            Self::Run => "run",
            Self::Search => "search",
            Self::Stop => "stop",
            Self::Timer => "timer",
            Self::Wand => "wand",
            Self::Warning => "warning",
        }
    }
}

impl From<Icon> for svg::Handle {
    fn from(icon: Icon) -> Self {
        icon.handle()
    }
}

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
struct Assets;
