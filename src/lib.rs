//! The reusable Bottles Next UI toolkit.
//!
//! The `next-ui` package also contains the desktop application, but this library
//! has no application or core dependencies. Callers provide controlled data and
//! semantic messages; widgets retain interaction mechanics such as focus,
//! capture, disclosure, popup placement, resizing, and animation.
//!
//! The supported facade is [`theme`], [`widget`], and [`Icon`].

mod icons;
pub mod theme;
mod widgets;

pub use icons::Icon;
/// The complete public widget catalog.
///
/// The caller owns values and semantic messages. Widgets own their interaction
/// state; keyed collections use caller-provided identities for their documented
/// behavior.
pub mod widget {
    pub use crate::widgets::{
        action_row::ActionRow,
        action_tile::ActionTile,
        artwork_card::{ArtworkCard, CardAction},
        button::{Button, ButtonKind},
        card::Card,
        cycle_row::CycleRow,
        dialog::{Dialog, WindowModal},
        expander_row::ExpanderRow,
        header_bar::HeaderBar,
        info_card::{InfoCard, Kind as InfoCardKind},
        info_row::InfoRow,
        list_row::ListRow,
        picker_row::PickerRow,
        popover::{Popover, PopoverItem},
        progress_ring::ProgressRing,
        row_group::RowGroup,
        search::{Search, SearchResult, SearchState},
        selector_row::SelectorRow,
        spacing,
        status_bar::{LogPanel, StatusBar},
        switcher::Switcher,
        switcher_row::SwitcherRow,
        tabs::{Tab, Tabs},
        text::TextExt,
        text_row::TextRow,
        title::Title,
    };
}
