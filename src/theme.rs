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

pub const BACKGROUND: Color = Color::from_rgb8(58, 50, 53);
pub const HINT: Color = Color::from_rgb8(41, 34, 37);
pub const DEEP_BACKGROUND: Color = Color::from_rgb8(27, 25, 26);
pub const SURFACE_DEEP: Color = Color::from_rgb8(54, 44, 49);
pub const PANEL: Color = BACKGROUND;
pub const SURFACE: Color = Color::from_rgb8(65, 57, 60);
pub const BORDER: Color = Color::from_rgb8(76, 66, 70);
pub const SURFACE_SELECTED: Color = Color::from_rgb8(89, 78, 82);

pub const MUTED: Color = Color::from_rgb8(166, 147, 154);
pub const SUBTLE: Color = Color::from_rgb8(210, 189, 197);
pub const TEXT: Color = Color::from_rgb8(250, 236, 241);
pub const ACCENT: Color = Color::from_rgb8(250, 230, 237);
pub const ACCENT_MUTED: Color = Color::from_rgb8(199, 172, 182);

pub const INFO: Color = Color::from_rgb8(51, 53, 71);
pub const ERROR: Color = Color::from_rgb8(92, 63, 63);
pub const WARNING: Color = Color::from_rgb8(107, 93, 71);
pub const SUCCESS: Color = Color::from_rgb8(53, 71, 51);
pub const WHITE: Color = Color::WHITE;

pub const SCRIM: Color = Color::from_rgba8(34, 28, 30, 171.0 / 255.0);

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
            base: pair(BACKGROUND, WHITE),
            weakest: pair(DEEP_BACKGROUND, TEXT),
            weaker: pair(PANEL, TEXT),
            weak: pair(SURFACE, TEXT),
            neutral: pair(BORDER, TEXT),
            strong: pair(BORDER, TEXT),
            stronger: pair(SURFACE_SELECTED, TEXT),
            strongest: pair(SURFACE_DEEP, TEXT),
        },
        primary: Primary {
            base: pair(MUTED, DEEP_BACKGROUND),
            weak: pair(SURFACE_SELECTED, DEEP_BACKGROUND),
            strong: pair(ACCENT_MUTED, DEEP_BACKGROUND),
        },
        secondary: Secondary {
            base: pair(DEEP_BACKGROUND, MUTED),
            weak: pair(PANEL, MUTED),
            strong: pair(SURFACE_DEEP, MUTED),
        },
        success: Success {
            base: pair(SUCCESS, WHITE),
            weak: pair(SUCCESS, WHITE),
            strong: pair(SUCCESS, WHITE),
        },
        warning: Warning {
            base: pair(WARNING, WHITE),
            weak: pair(WARNING, WHITE),
            strong: pair(WARNING, WHITE),
        },
        danger: Danger {
            base: pair(ERROR, WHITE),
            weak: pair(ERROR, WHITE),
            strong: pair(ERROR, WHITE),
        },
        is_dark: true,
    })
}

pub const fn info() -> Pair {
    pair(INFO, WHITE)
}

pub const fn hint() -> Pair {
    pair(HINT, WHITE)
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}

#[cfg(test)]
mod tests {
    use super::{
        ACCENT, BACKGROUND, BORDER, DEEP_BACKGROUND, MUTED, SURFACE, WHITE, theme as bottles_theme,
    };

    #[test]
    fn iced_roles_use_the_extracted_palette() {
        let theme = bottles_theme();
        let colors = theme.extended_palette();

        assert_eq!(theme.palette().background, BACKGROUND);
        assert_eq!(theme.palette().primary, ACCENT);
        assert_eq!(colors.background.weak.color, SURFACE);
        assert_eq!(colors.background.neutral.color, BORDER);
        assert_eq!(colors.primary.base.color, MUTED);
        assert_eq!(colors.secondary.base.color, DEEP_BACKGROUND);
        assert_eq!(colors.danger.base.text, WHITE);
    }
}
