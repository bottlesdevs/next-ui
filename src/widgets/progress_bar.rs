use iced::{
    Alignment, Background, Border, Element, Fill,
    widget::{Space, column, progress_bar, row, text},
};

use super::{spacing, text::TextExt as _, text_size};

const THICKNESS: f32 = 4.0;

/// A thin progress track, optionally under a caption line: what the bar is
/// for, and its percentage at the end. The track has a line of its own, so
/// a caption that changes never changes its width.
pub struct ProgressBar<'a, Message> {
    progress: f32,
    caption: Option<Element<'a, Message>>,
    percentage: bool,
}

impl<'a, Message: 'a> ProgressBar<'a, Message> {
    /// Creates a track filled to `progress`, clamped to `0.0..=1.0`.
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            caption: None,
            percentage: false,
        }
    }

    /// Shows `caption` above the track.
    pub fn caption(mut self, caption: impl Into<Element<'a, Message>>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Shows the rounded percentage at the end of the caption line.
    pub fn percentage(mut self) -> Self {
        self.percentage = true;
        self
    }
}

impl<'a, Message: 'a> From<ProgressBar<'a, Message>> for Element<'a, Message> {
    fn from(bar: ProgressBar<'a, Message>) -> Self {
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

        if bar.caption.is_none() && !bar.percentage {
            return track.into();
        }

        let caption = bar
            .caption
            .unwrap_or_else(|| Space::new().width(Fill).into());
        let mut line = row![iced::widget::container(caption).width(Fill)]
            .spacing(spacing::SM)
            .align_y(Alignment::Center);
        if bar.percentage {
            line = line.push(
                text(format!("{:.0}%", bar.progress * 100.0))
                    .size(text_size::DENSE)
                    .medium(),
            );
        }

        column![line, track].spacing(spacing::XS).into()
    }
}
