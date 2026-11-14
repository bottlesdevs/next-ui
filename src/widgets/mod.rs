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

/// Shared corner radii.
pub mod radius {
    /// A single row, card or control.
    pub const ROW: f32 = 6.0;
    /// A box that holds rows or floats over the page, e.g. a popup panel or
    /// a notification.
    pub const BOX: f32 = 8.0;
    /// Fully rounded ends; larger than any control so it always caps at half
    /// the height.
    pub const PILL: f32 = 999.0;
}

/// Shared text sizes, smallest first. The design uses many close steps;
/// each is named by where it is used.
pub mod text_size {
    /// Status bar lines and a compact title's subtitle.
    pub const CAPTION: f32 = 12.0;
    /// Tags, section labels and version numbers.
    pub const FOOTNOTE: f32 = 13.0;
    /// A supporting line under a heading, e.g. a popover's or banner's body.
    pub const SMALL: f32 = 14.0;
    /// Text in dense rows and cards: options, descriptions, row buttons.
    pub const DENSE: f32 = 15.0;
    /// Row subtitles and dialog bodies; Iced's default size.
    pub const DETAIL: f32 = 16.0;
    /// Titles of cards, popovers and banners; page descriptions.
    pub const HEADING: f32 = 17.0;
    /// Row titles and button labels.
    pub const LABEL: f32 = 18.0;
    /// A lead paragraph.
    pub const BODY: f32 = 20.0;
    pub const DIALOG_TITLE: f32 = 21.0;
    /// A group's heading.
    pub const SUBTITLE: f32 = 22.0;
    /// A detail page's name and a header title with a status.
    pub const HEADLINE: f32 = 24.0;
    /// The title of an empty or status page.
    pub const PAGE_TITLE: f32 = 26.0;
    pub const TITLE: f32 = 28.0;
    /// A first-run heading and a header title on its own.
    pub const DISPLAY: f32 = 32.0;
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
pub(crate) mod inert;
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
pub mod size_group;
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
pub mod window_handle;
