mod icons;
pub mod theme;
mod ui;
mod widgets;

pub use icons::Icon;
pub use ui::chrome as window;

/// Reusable application widgets.
pub mod widget {
    pub use crate::widgets::{
        action_row::{ActionRow, State as ActionRowState},
        artwork_card::{ArtworkCard, CardAction},
        button::{Button, ButtonKind},
        card::Card,
        cycle_row::CycleRow,
        dialog::{Dialog, WindowModal},
        drop_target::DropTarget,
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
        status_bar::{BottleStatus, StatusBar},
        switcher::Switcher,
        switcher_row::SwitcherRow,
        tabs::{Tab, Tabs},
        text::TextExt,
        text_row::TextRow,
        title::Title,
    };
}
