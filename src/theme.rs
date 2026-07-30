use iced::{
    Color, Theme,
    theme::{
        Palette,
        palette::{
            Background as BackgroundPalette, Danger, Extended, Pair, Primary, Secondary, Success,
            Warning,
        },
    },
};

pub const PANEL: Color = Color::from_rgb8(58, 50, 53);
pub const OVERLAY: Color = Color::from_rgba8(105, 89, 95, 0.31);
pub const FOCUS: Color = Color::from_rgb8(151, 71, 255);
pub const SURFACE: Color = Color::from_rgb8(65, 57, 60);
pub const BACKGROUND: Color = Color::from_rgb8(41, 34, 37);
pub const SURFACE_HOVERED: Color = Color::from_rgb8(77, 66, 70);
pub const MUTED: Color = Color::from_rgb8(166, 147, 154);
pub const TEXT: Color = Color::from_rgb8(250, 236, 241);
pub const BORDER: Color = Color::from_rgb8(76, 66, 70);
pub const ACCENT: Color = Color::from_rgb8(250, 230, 237);
pub const ACCENT_TRANSPARENT: Color = Color::from_rgba8(250, 230, 237, 0.0);
pub const SUBTLE: Color = Color::from_rgb8(210, 189, 197);
pub const BORDER_TRANSLUCENT: Color = Color::from_rgba8(76, 66, 70, 0.44);
pub const DEEP_BACKGROUND: Color = Color::from_rgb8(27, 25, 26);
pub const SURFACE_SELECTED: Color = Color::from_rgb8(89, 78, 82);
pub const SCRIM: Color = Color::from_rgba8(34, 28, 31, 0.67);
pub const BACKGROUND_SCRIM: Color = Color::from_rgba8(41, 34, 37, 0.67);
pub const INFO: Color = Color::from_rgb8(65, 91, 105);
pub const WHITE: Color = Color::WHITE;
pub const INFO_DARK: Color = Color::from_rgb8(51, 53, 71);
pub const ERROR: Color = Color::from_rgb8(92, 63, 63);
pub const WARNING: Color = Color::from_rgb8(107, 93, 71);
pub const SUCCESS: Color = Color::from_rgb8(53, 71, 51);
pub const ACCENT_MUTED: Color = Color::from_rgb8(199, 172, 182);
pub const NEUTRAL: Color = Color::from_rgb8(68, 68, 68);
pub const WHITE_TINT: Color = Color::from_rgba8(255, 255, 255, 0.10);
pub const SURFACE_DEEP: Color = Color::from_rgb8(54, 44, 49);
pub const BACKGROUND_TRANSPARENT: Color = Color::from_rgba8(41, 34, 37, 0.0);
pub const SURFACE_LIGHT: Color = Color::from_rgb8(82, 71, 75);
pub const TEAL: Color = Color::from_rgb8(27, 184, 175);
pub const TEAL_HOVERED: Color = Color::from_rgb8(24, 177, 163);
pub const SURFACE_BRIGHT: Color = Color::from_rgb8(114, 99, 105);

pub fn theme() -> Theme {
    let palette = Palette {
        background: BACKGROUND,
        text: TEXT,
        primary: ACCENT,
        success: SUCCESS,
        warning: WARNING,
        danger: ERROR,
    };

    Theme::custom_with_fn("Bottles Next", palette, |_| Extended {
        background: BackgroundPalette {
            base: pair(BACKGROUND, TEXT),
            weakest: pair(SURFACE_DEEP, TEXT),
            weaker: pair(PANEL, TEXT),
            weak: pair(SURFACE, TEXT),
            neutral: pair(BORDER, TEXT),
            strong: pair(SURFACE_HOVERED, TEXT),
            stronger: pair(SURFACE_SELECTED, TEXT),
            strongest: pair(SURFACE_BRIGHT, TEXT),
        },
        primary: Primary {
            base: pair(ACCENT, BACKGROUND),
            weak: pair(ACCENT_MUTED, BACKGROUND),
            strong: pair(TEXT, BACKGROUND),
        },
        secondary: Secondary {
            base: pair(SURFACE, TEXT),
            weak: pair(PANEL, MUTED),
            strong: pair(SURFACE_HOVERED, TEXT),
        },
        success: Success {
            base: pair(SUCCESS, TEXT),
            weak: pair(SUCCESS, TEXT),
            strong: pair(SUCCESS, TEXT),
        },
        warning: Warning {
            base: pair(WARNING, TEXT),
            weak: pair(WARNING, TEXT),
            strong: pair(WARNING, TEXT),
        },
        danger: Danger {
            base: pair(ERROR, TEXT),
            weak: pair(ERROR, TEXT),
            strong: pair(ERROR, TEXT),
        },
        is_dark: true,
    })
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}
