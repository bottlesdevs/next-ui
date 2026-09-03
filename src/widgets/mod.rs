pub mod action_tile;
mod anchored_overlay;
mod control;
mod focusable_action;
mod list_navigation;
mod menu;
mod style;
mod surface;

use iced::{ContentFit, Point, Rectangle, Size, Theme, advanced::svg::Renderer as _};

/// Shared spacing scale used by toolkit widgets and application layouts.
pub mod spacing {
    pub const XS: f32 = 6.0;
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 18.0;
    pub const LG: f32 = 24.0;
    pub const XG: f32 = 32.0;
}

fn draw_caret(renderer: &mut iced::Renderer, theme: &Theme, slot: Rectangle, expansion: f32) {
    let handle = crate::icons::Icon::DownCaret.handle();
    let Size { width, height } = renderer.measure_svg(&handle);
    let size = ContentFit::Contain.fit(Size::new(width as f32, height as f32), slot.size());
    let bounds = Rectangle::new(
        Point::new(
            slot.center_x() - size.width / 2.0,
            slot.center_y() - size.height / 2.0,
        ),
        size,
    );

    renderer.draw_svg(
        iced::advanced::svg::Svg {
            handle,
            color: Some(crate::theme::colors(theme).muted),
            rotation: (std::f32::consts::PI * expansion).into(),
            opacity: 1.0,
        },
        bounds,
        slot,
    );
}

pub mod action_row;
pub mod artwork_card;
pub mod button;
pub mod card;
pub mod cycle_row;
pub mod dialog;
pub mod expander_row;
pub mod header_bar;
pub mod info_card;
pub mod info_row;
pub mod list_row;
pub mod picker_row;
pub mod popover;
pub mod progress_ring;
pub mod row_group;
pub mod search;
pub mod selector_row;
pub mod status_bar;
pub mod switcher;
pub mod switcher_row;
pub mod tabs;
pub mod text;
pub mod text_row;
pub mod title;
