use iced::{
    Background, Border, Color, Theme,
    theme::{
        Mode, Palette, Style as ApplicationStyle,
        palette::{
            Background as BackgroundPalette, Danger, Extended, Pair, Primary, Secondary, Success,
            Warning,
        },
    },
    widget::{container, scrollable},
};

pub const BACKGROUND: Color = Color::from_rgb8(58, 50, 53);
pub const WINDOW: Color = Color::from_rgb8(41, 34, 37);
pub const WINDOW_BORDER: Color = Color::from_rgb8(54, 44, 49);
pub const HINT: Color = WINDOW;
pub const DEEP_BACKGROUND: Color = Color::from_rgb8(27, 25, 26);
pub const SURFACE_DEEP: Color = WINDOW_BORDER;
pub const PANEL: Color = BACKGROUND;
pub const SURFACE: Color = Color::from_rgb8(65, 57, 60);
pub const BORDER: Color = Color::from_rgb8(76, 66, 70);
pub const ROW_HOVER_STRONG: Color = Color::from_rgb8(96, 85, 89);
pub const SURFACE_SELECTED: Color = Color::from_rgb8(89, 78, 82);

pub const MUTED: Color = Color::from_rgb8(166, 147, 154);
pub const TEXT: Color = Color::from_rgb8(250, 236, 241);
pub const ACCENT: Color = Color::from_rgb8(250, 230, 237);
pub const ACCENT_MUTED: Color = Color::from_rgb8(199, 172, 182);

pub const INFO: Color = Color::from_rgb8(51, 53, 71);
pub const ERROR: Color = Color::from_rgb8(92, 63, 63);
pub const WARNING: Color = Color::from_rgb8(107, 93, 71);
pub const SUCCESS: Color = Color::from_rgb8(53, 71, 51);
pub const WHITE: Color = Color::WHITE;

pub const SCRIM: Color = Color::from_rgba8(34, 28, 30, 171.0 / 255.0);

pub const BACKGROUND_LIGHT: Color = Color::from_rgb8(247, 242, 243);
pub const WINDOW_LIGHT: Color = Color::from_rgb8(255, 253, 254);
pub const WINDOW_BORDER_LIGHT: Color = Color::from_rgb8(226, 215, 219);
pub const HINT_LIGHT: Color = WINDOW_LIGHT;
pub const DEEP_BACKGROUND_LIGHT: Color = Color::from_rgb8(251, 248, 249);
pub const SURFACE_DEEP_LIGHT: Color = Color::from_rgb8(214, 200, 204);
pub const PANEL_LIGHT: Color = BACKGROUND_LIGHT;
pub const SURFACE_LIGHT: Color = Color::from_rgb8(238, 231, 233);
pub const BORDER_LIGHT: Color = Color::from_rgb8(223, 212, 216);
pub const ROW_HOVER_STRONG_LIGHT: Color = Color::from_rgb8(206, 190, 195);
pub const SURFACE_SELECTED_LIGHT: Color = Color::from_rgb8(216, 201, 206);

pub const MUTED_LIGHT: Color = Color::from_rgb8(135, 116, 122);
pub const TEXT_LIGHT: Color = Color::from_rgb8(36, 28, 31);
pub const ACCENT_LIGHT: Color = Color::from_rgb8(168, 76, 104);
pub const ACCENT_MUTED_LIGHT: Color = Color::from_rgb8(196, 150, 164);

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

pub fn light_theme() -> Theme {
    let palette = Palette {
        background: BACKGROUND_LIGHT,
        text: TEXT_LIGHT,
        primary: ACCENT_LIGHT,
        success: SUCCESS,
        warning: WARNING,
        danger: ERROR,
    };

    Theme::custom_with_fn("Bottles Next Light", palette, |_| Extended {
        background: BackgroundPalette {
            base: pair(BACKGROUND_LIGHT, TEXT_LIGHT),
            weakest: pair(DEEP_BACKGROUND_LIGHT, TEXT_LIGHT),
            weaker: pair(PANEL_LIGHT, TEXT_LIGHT),
            weak: pair(SURFACE_LIGHT, TEXT_LIGHT),
            neutral: pair(BORDER_LIGHT, TEXT_LIGHT),
            strong: pair(BORDER_LIGHT, TEXT_LIGHT),
            stronger: pair(SURFACE_SELECTED_LIGHT, TEXT_LIGHT),
            strongest: pair(SURFACE_DEEP_LIGHT, TEXT_LIGHT),
        },
        primary: Primary {
            base: pair(MUTED_LIGHT, WHITE),
            weak: pair(SURFACE_SELECTED_LIGHT, TEXT_LIGHT),
            strong: pair(ACCENT_MUTED_LIGHT, TEXT_LIGHT),
        },
        secondary: Secondary {
            base: pair(WINDOW_LIGHT, MUTED_LIGHT),
            weak: pair(PANEL_LIGHT, MUTED_LIGHT),
            strong: pair(SURFACE_DEEP_LIGHT, MUTED_LIGHT),
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
        is_dark: false,
    })
}

/// Colors we need that don't fit anywhere in iced's built-in [`Extended`]
/// palette shape — window chrome, hint panels, the scrollbar thumb, and a
/// stronger row-hover accent.
/// `Extended` can't be extended with new fields (it's a fixed struct baked
/// into `Theme::Custom`), so this wraps the inner [`Theme`] instead: iced
/// still only ever sees plain `Theme` values, while call sites that need one
/// of these extra colors go through `BottlesTheme`.
#[derive(Debug, Clone)]
pub struct BottlesTheme {
    pub theme: Theme,
    pub window: Color,
    pub window_border: Color,
    pub panel: Color,
    pub hint: Pair,
    pub row_hover_strong: Color,
    pub muted: Color,
}

impl BottlesTheme {
    pub fn dark() -> Self {
        Self {
            theme: theme(),
            window: WINDOW,
            window_border: WINDOW_BORDER,
            panel: PANEL,
            hint: pair(HINT, WHITE),
            row_hover_strong: ROW_HOVER_STRONG,
            muted: MUTED,
        }
    }

    pub fn light() -> Self {
        Self {
            theme: light_theme(),
            window: WINDOW_LIGHT,
            window_border: WINDOW_BORDER_LIGHT,
            panel: PANEL_LIGHT,
            hint: pair(HINT_LIGHT, TEXT_LIGHT),
            row_hover_strong: ROW_HOVER_STRONG_LIGHT,
            muted: MUTED_LIGHT,
        }
    }

    /// Builds the [`BottlesTheme`] for the given system [`Mode`], defaulting
    /// to dark when the system preference can't be determined (`Mode::None`).
    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self::light(),
            Mode::Dark | Mode::None => Self::dark(),
        }
    }
}

impl std::ops::Deref for BottlesTheme {
    type Target = Theme;

    fn deref(&self) -> &Theme {
        &self.theme
    }
}

/// Recovers a [`BottlesTheme`] from a plain [`Theme`] — for the widget
/// style closures iced itself invokes with `&Theme`, which have no way to
/// receive our richer type directly.
impl From<&Theme> for BottlesTheme {
    fn from(theme: &Theme) -> Self {
        if theme.extended_palette().is_dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

pub(crate) fn window(bottles_theme: &BottlesTheme) -> container::Style {
    container::Style {
        background: Some(Background::Color(bottles_theme.window)),
        border: Border::default()
            .rounded(12)
            .color(bottles_theme.window_border)
            .width(1),
        ..container::Style::default()
    }
}

pub fn application(theme: &Theme) -> ApplicationStyle {
    ApplicationStyle {
        background_color: Color::TRANSPARENT,
        text_color: theme.palette().text,
    }
}

pub fn panel(theme: &Theme) -> container::Style {
    let bottles_theme = BottlesTheme::from(theme);

    container::Style {
        background: Some(Background::Color(bottles_theme.panel)),
        border: Border::default().rounded(11),
        ..container::Style::default()
    }
}

pub fn scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let bottles_theme = BottlesTheme::from(theme);
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(bottles_theme.muted),
            border: Border::default().rounded(999),
        },
    };

    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

pub(crate) const fn info() -> Pair {
    pair(INFO, WHITE)
}

const fn pair(color: Color, text: Color) -> Pair {
    Pair { color, text }
}
