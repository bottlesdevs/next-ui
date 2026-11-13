use iced::{
    Background, Border, Element, Fill,
    widget::{progress_bar, row, text},
};

use super::text_size;

use super::text::TextExt as _;

const THICKNESS: f32 = 4.0;

/// A thin progress track, optionally followed by its percentage.
pub struct ProgressBar {
    progress: f32,
    percentage: bool,
}

impl ProgressBar {
    /// Creates a track filled to `progress`, clamped to `0.0..=1.0`.
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            percentage: false,
        }
    }

    /// Shows the rounded percentage after the track.
    pub fn percentage(mut self) -> Self {
        self.percentage = true;
        self
    }
}

impl<'a, Message: 'a> From<ProgressBar> for Element<'a, Message> {
    fn from(bar: ProgressBar) -> Self {
        let track = progress_bar(0.0..=1.0, bar.progress)
            .length(Fill)
            .girth(THICKNESS)
            .style(|theme| {
                let colors = crate::theme::colors(theme);

                progress_bar::Style {
                    background: Background::Color(colors.sunken),
                    bar: Background::Color(colors.accent),
                    border: Border::default().rounded(THICKNESS / 2.0),
                }
            });

        if bar.percentage {
            row![
                track,
                text(format!("{:.0}%", bar.progress * 100.0))
                    .size(text_size::SMALL)
                    .muted()
            ]
            .spacing(10)
            .align_y(iced::Center)
            .into()
        } else {
            track.into()
        }
    }
}
