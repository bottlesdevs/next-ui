//! The reusable Bottles Next UI toolkit.
//!
//! The `next-ui` package also contains the desktop application, but this library
//! facade exposes no application or domain types. Callers provide controlled
//! data and semantic messages; widgets retain only cohesive interaction state
//! such as focus, disclosure, popup placement, resizing, and animation.
//!
//! The supported facade is [`theme`], [`widget`], [`Icon`], the window
//! [`chrome`], and the adaptive [`layout`].

pub mod chrome;
mod icons;
pub mod layout;
pub mod theme;
mod widgets;

pub use icons::Icon;
/// The complete public widget catalog.
///
/// The caller owns values and semantic messages. Widgets own their interaction
/// state.
pub mod widget {
    pub use crate::widgets::{
        action_row::ActionRow,
        action_tile::ActionTile,
        artwork_card::{ArtworkCard, CardAction},
        avatar::{Avatar, AvatarShape},
        boxed_group::BoxedGroup,
        button::{Button, ButtonKind},
        card::Card,
        cycle_row::CycleRow,
        dialog::{Dialog, WindowModal},
        expander_row::ExpanderRow,
        filter_chips::FilterChips,
        header,
        header_bar::{HeaderBar, header_button},
        info_card::{InfoCard, Kind as InfoCardKind},
        info_row::InfoRow,
        list_row::ListRow,
        notification::Notification,
        picker_row::PickerRow,
        popover::Popover,
        progress_bar::ProgressBar,
        progress_ring::ProgressRing,
        radius,
        row_group::RowGroup,
        search::{Search, SearchState},
        selector_row::SelectorRow,
        spacing,
        status_bar::{LogPanel, StatusBar},
        status_page::StatusPage,
        switcher::Switcher,
        switcher_row::SwitcherRow,
        tabs::{Tab, Tabs},
        tag::Tag,
        text::TextExt,
        text_row::TextRow,
        text_size,
        title::Title,
    };
}
