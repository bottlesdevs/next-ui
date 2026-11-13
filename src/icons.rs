use iced::{Theme, widget::svg};
use rust_embed::RustEmbed;

pub(crate) const SIZE: f32 = 24.0;

/// A typed reference to an icon embedded in the toolkit with `rust-embed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Arrow,
    ArrowRight,
    Bottles,
    Bug,
    Checkmark,
    ChevronRight,
    Chip,
    Code,
    Computer,
    Controller,
    Copy,
    Cross,
    Custom,
    Disk,
    Document,
    DoubleCheckmark,
    DownCaret,
    Download,
    EllipsisVertical,
    Error,
    ExternalLink,
    Folder,
    Gear,
    HollowGear,
    Info,
    Library,
    Lightning,
    Lock,
    Pencil,
    People,
    Person,
    Play,
    Plus,
    Power,
    Refresh,
    Run,
    RunProgram,
    Search,
    Stop,
    Timer,
    Trash,
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
            Self::ArrowRight => "arrow_right",
            Self::Bottles => "bottles-symbolic",
            Self::Bug => "bug",
            Self::Checkmark => "checkmark",
            Self::ChevronRight => "chevron_right",
            Self::Chip => "chip",
            Self::Code => "code",
            Self::Computer => "computer",
            Self::Controller => "controller",
            Self::Copy => "copy",
            Self::Cross => "cross",
            Self::Custom => "custom",
            Self::Disk => "disk",
            Self::Document => "document",
            Self::DoubleCheckmark => "double_checkmark",
            Self::DownCaret => "down_caret",
            Self::Download => "download",
            Self::EllipsisVertical => "ellipsis_vertical",
            Self::Error => "error",
            Self::ExternalLink => "external_link",
            Self::Folder => "folder",
            Self::Gear => "gear",
            Self::HollowGear => "hollow-gear",
            Self::Info => "info",
            Self::Library => "library",
            Self::Lightning => "lightning",
            Self::Lock => "lock",
            Self::Pencil => "pencil",
            Self::People => "people",
            Self::Person => "person",
            Self::Play => "play",
            Self::Plus => "plus",
            Self::Power => "power",
            Self::Refresh => "refresh",
            Self::Run => "run",
            Self::RunProgram => "run_program",
            Self::Search => "search",
            Self::Stop => "stop",
            Self::Timer => "timer",
            Self::Trash => "trash",
            Self::Wand => "wand",
            Self::Warning => "warning",
        }
    }
}

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
struct Assets;

/// An icon drawn in the text colour it inherits, so it follows the state of
/// the control it sits in.
pub(crate) struct Inherited {
    handle: svg::Handle,
    size: f32,
}

impl Icon {
    pub(crate) fn inherited(self, size: f32) -> Inherited {
        Inherited {
            handle: self.handle(),
            size,
        }
    }
}

impl<Message> iced::advanced::Widget<Message, Theme, iced::Renderer> for Inherited {
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size::new(self.size.into(), self.size.into())
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &iced::Renderer,
        _limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(iced::Size::new(self.size, self.size))
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        use iced::advanced::svg::Renderer as _;

        renderer.draw_svg(
            iced::advanced::svg::Svg {
                handle: self.handle.clone(),
                color: Some(style.text_color),
                rotation: 0.0.into(),
                opacity: 1.0,
            },
            layout.bounds(),
            layout.bounds(),
        );
    }
}

impl<'a, Message> From<Inherited> for iced::Element<'a, Message> {
    fn from(icon: Inherited) -> Self {
        Self::new(icon)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const ALL: [Icon; 43] = [
        Icon::Arrow,
        Icon::ArrowRight,
        Icon::Bottles,
        Icon::Bug,
        Icon::Checkmark,
        Icon::ChevronRight,
        Icon::Chip,
        Icon::Code,
        Icon::Computer,
        Icon::Controller,
        Icon::Copy,
        Icon::Cross,
        Icon::Custom,
        Icon::Disk,
        Icon::Document,
        Icon::DoubleCheckmark,
        Icon::DownCaret,
        Icon::Download,
        Icon::EllipsisVertical,
        Icon::Error,
        Icon::ExternalLink,
        Icon::Folder,
        Icon::Gear,
        Icon::HollowGear,
        Icon::Info,
        Icon::Library,
        Icon::Lightning,
        Icon::Lock,
        Icon::Pencil,
        Icon::People,
        Icon::Person,
        Icon::Play,
        Icon::Plus,
        Icon::Power,
        Icon::Refresh,
        Icon::Run,
        Icon::RunProgram,
        Icon::Search,
        Icon::Stop,
        Icon::Timer,
        Icon::Trash,
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
