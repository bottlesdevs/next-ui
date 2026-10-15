use iced::{Theme, widget::svg};
use rust_embed::RustEmbed;

pub(crate) const SIZE: f32 = 24.0;

/// A typed reference to an icon embedded in the toolkit with `rust-embed`.
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
    pub(crate) fn handle(self) -> svg::Handle {
        let path = format!("icons/{}.svg", self.name());
        let icon = Assets::get(&path)
            .unwrap_or_else(|| unreachable!("typed embedded icon is missing: {path}"));

        svg::Handle::from_memory(icon.data)
    }

    /// Builds an SVG widget using the current semantic muted color.
    pub fn view<'a>(self) -> svg::Svg<'a> {
        svg(self.handle())
            .width(SIZE)
            .height(SIZE)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(crate::theme::muted(theme)),
            })
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

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
struct Assets;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const ALL: [Icon; 29] = [
        Icon::Arrow,
        Icon::Bottles,
        Icon::Checkmark,
        Icon::Chip,
        Icon::Computer,
        Icon::Controller,
        Icon::Cross,
        Icon::Custom,
        Icon::Disk,
        Icon::DoubleCheckmark,
        Icon::DownCaret,
        Icon::EllipsisVertical,
        Icon::Error,
        Icon::Folder,
        Icon::Gear,
        Icon::HollowGear,
        Icon::Info,
        Icon::Lightning,
        Icon::Pencil,
        Icon::Person,
        Icon::Play,
        Icon::Plus,
        Icon::Power,
        Icon::Run,
        Icon::Search,
        Icon::Stop,
        Icon::Timer,
        Icon::Wand,
        Icon::Warning,
    ];

    #[test]
    fn every_typed_icon_resolves_and_every_embedded_icon_is_typed() {
        let declared: BTreeSet<_> = ALL
            .into_iter()
            .map(|icon| {
                let path = format!("icons/{}.svg", icon.name());
                assert!(
                    Assets::get(&path).is_some(),
                    "missing embedded icon: {path}"
                );
                path
            })
            .collect();
        let embedded: BTreeSet<_> = Assets::iter()
            .filter(|path| path.starts_with("icons/") && path.ends_with(".svg"))
            .map(|path| path.into_owned())
            .collect();

        assert_eq!(
            declared.len(),
            ALL.len(),
            "two Icon variants share an asset"
        );
        assert_eq!(declared, embedded);
    }
}
