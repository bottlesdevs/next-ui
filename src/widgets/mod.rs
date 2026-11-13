pub mod action_tile;
mod anchored_overlay;
mod disclosure;
pub(crate) mod focus;
mod focusable_action;
mod popup;
mod style;

/// Shared spacing scale used by toolkit widgets and application layouts.
pub mod spacing {
    pub const XS: f32 = 6.0;
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 18.0;
    pub const LG: f32 = 24.0;
    pub const XG: f32 = 32.0;
}

/// Shared dimensions for header content and frame-owned controls.
pub mod header {
    pub const HEIGHT: f32 = 64.0;
    pub const INSET: f32 = super::spacing::SM;
    pub const CONTROL: f32 = 32.0;
    pub const ICON: f32 = 16.0;
}

pub mod action_row;
pub mod artwork_card;
pub mod avatar;
pub mod boxed_group;
pub mod button;
pub mod card;
pub mod cycle_row;
pub mod dialog;
pub mod expander_row;
pub mod filter_chips;
pub mod header_bar;
pub mod info_card;
pub mod info_row;
pub mod list_row;
pub mod notification;
pub mod picker_row;
pub mod popover;
pub mod progress_bar;
pub mod progress_ring;
pub mod row_group;
pub mod search;
pub mod selector_row;
pub mod status_bar;
pub mod status_page;
pub mod switcher;
pub mod switcher_row;
pub mod tabs;
pub mod tag;
pub mod text;
mod text_field;
pub mod text_row;
pub mod title;
