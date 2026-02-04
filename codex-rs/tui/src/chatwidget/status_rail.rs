use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;

use crate::render::renderable::Renderable;

const SAMPLE_INTERVAL: Duration = Duration::from_millis(750);
const SPARKLINE_WIDTH: usize = 8;
const SPINNER_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
const SPARK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct StatusRailSample {
    pub(crate) cpu_percent: Option<f32>,
    pub(crate) memory_percent: Option<f32>,
    pub(crate) gpu_percent: Option<f32>,
    pub(crate) tokens_per_second: Option<f32>,
    pub(crate) context_remaining_percent: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct StatusRailView {
    pub(crate) phase: usize,
    pub(crate) cpu_percent: Option<f32>,
    pub(crate) memory_percent: Option<f32>,
    pub(crate) gpu_percent: Option<f32>,
    pub(crate) tokens_per_second: Option<f32>,
    pub(crate) context_remaining_percent: Option<i64>,
    pub(crate) cpu_sparkline: String,
    pub(crate) memory_sparkline: String,
    pub(crate) gpu_sparkline: String,
    pub(crate) tokens_sparkline: String,
    pub(crate) context_sparkline: String,
}

impl Default for StatusRailView {
    fn default() -> Self {
        Self {
            phase: 0,
            cpu_percent: None,
            memory_percent: None,
            gpu_percent: None,
            tokens_per_second: None,
            context_remaining_percent: None,
            cpu_sparkline: "·".repeat(SPARKLINE_WIDTH),
            memory_sparkline: "·".repeat(SPARKLINE_WIDTH),
            gpu_sparkline: "·".repeat(SPARKLINE_WIDTH),
            tokens_sparkline: "·".repeat(SPARKLINE_WIDTH),
            context_sparkline: "·".repeat(SPARKLINE_WIDTH),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct StatusRailTracker {
    phase: usize,
    last_sample_at: Option<Instant>,
    cpu_history: VecDeque<Option<f32>>,
    memory_history: VecDeque<Option<f32>>,
    gpu_history: VecDeque<Option<f32>>,
    tokens_history: VecDeque<Option<f32>>,
    context_history: VecDeque<Option<f32>>,
}

impl StatusRailTracker {
    pub(crate) fn observe(&mut self, sample: StatusRailSample) -> StatusRailView {
        let now = Instant::now();
        let should_record = self
            .last_sample_at
            .is_none_or(|last| now.saturating_duration_since(last) >= SAMPLE_INTERVAL)
            || self.cpu_history.is_empty();
        if should_record {
            self.last_sample_at = Some(now);
            Self::push_sample(&mut self.cpu_history, sample.cpu_percent);
            Self::push_sample(&mut self.memory_history, sample.memory_percent);
            Self::push_sample(&mut self.gpu_history, sample.gpu_percent);
            Self::push_sample(&mut self.tokens_history, sample.tokens_per_second);
            Self::push_sample(
                &mut self.context_history,
                sample.context_remaining_percent.map(|value| value as f32),
            );
        }

        self.phase = (self.phase + 1) % SPINNER_FRAMES.len();

        StatusRailView {
            phase: self.phase,
            cpu_percent: sample.cpu_percent,
            memory_percent: sample.memory_percent,
            gpu_percent: sample.gpu_percent,
            tokens_per_second: sample.tokens_per_second,
            context_remaining_percent: sample.context_remaining_percent,
            cpu_sparkline: Self::build_percent_sparkline(&self.cpu_history),
            memory_sparkline: Self::build_percent_sparkline(&self.memory_history),
            gpu_sparkline: Self::build_percent_sparkline(&self.gpu_history),
            tokens_sparkline: Self::build_speed_sparkline(&self.tokens_history),
            context_sparkline: Self::build_percent_sparkline(&self.context_history),
        }
    }

    fn push_sample(history: &mut VecDeque<Option<f32>>, sample: Option<f32>) {
        if history.len() >= SPARKLINE_WIDTH {
            history.pop_front();
        }
        history.push_back(sample);
    }

    fn build_percent_sparkline(history: &VecDeque<Option<f32>>) -> String {
        Self::build_sparkline(history, 100.0)
    }

    fn build_speed_sparkline(history: &VecDeque<Option<f32>>) -> String {
        let max_speed = history
            .iter()
            .flatten()
            .copied()
            .fold(0.0_f32, f32::max)
            .max(1.0);
        Self::build_sparkline(history, max_speed)
    }

    fn build_sparkline(history: &VecDeque<Option<f32>>, max_value: f32) -> String {
        let mut line = String::with_capacity(SPARKLINE_WIDTH);
        for _ in 0..SPARKLINE_WIDTH.saturating_sub(history.len()) {
            line.push('·');
        }
        for sample in history {
            line.push(Self::spark_char(*sample, max_value));
        }
        line
    }

    fn spark_char(sample: Option<f32>, max_value: f32) -> char {
        let Some(sample) = sample else {
            return '·';
        };
        if max_value <= 0.0 {
            return SPARK_CHARS[0];
        }
        let ratio = (sample.max(0.0) / max_value).clamp(0.0, 1.0);
        let index = ((SPARK_CHARS.len() - 1) as f32 * ratio).round() as usize;
        SPARK_CHARS[index.min(SPARK_CHARS.len() - 1)]
    }
}

pub(crate) struct StatusRail {
    view: StatusRailView,
}

impl StatusRail {
    pub(crate) fn new(view: StatusRailView) -> Self {
        Self { view }
    }

    fn wide_line(&self) -> Line<'static> {
        Line::from(vec![
            SPINNER_FRAMES[self.view.phase].cyan(),
            " live ".dim(),
            "CPU ".dim(),
            self.view.cpu_sparkline.clone().cyan(),
            " ".into(),
            Self::colored_percent(self.view.cpu_percent),
            "  MEM ".dim(),
            self.view.memory_sparkline.clone().blue(),
            " ".into(),
            Self::colored_percent(self.view.memory_percent),
            "  GPU ".dim(),
            self.view.gpu_sparkline.clone().magenta(),
            " ".into(),
            Self::colored_percent(self.view.gpu_percent),
            "  speed ".dim(),
            self.view.tokens_sparkline.clone().green(),
            " ".into(),
            Self::colored_token_speed(self.view.tokens_per_second),
            "  space ".dim(),
            self.view.context_sparkline.clone().cyan(),
            " ".into(),
            Self::colored_context_percent(self.view.context_remaining_percent),
        ])
    }

    fn compact_line(&self) -> Line<'static> {
        Line::from(vec![
            SPINNER_FRAMES[self.view.phase].cyan(),
            " CPU ".dim(),
            self.view.cpu_sparkline.clone().cyan(),
            " ".into(),
            Self::colored_percent(self.view.cpu_percent),
            " MEM ".dim(),
            self.view.memory_sparkline.clone().blue(),
            " ".into(),
            Self::colored_percent(self.view.memory_percent),
            " GPU ".dim(),
            self.view.gpu_sparkline.clone().magenta(),
            " ".into(),
            Self::colored_percent(self.view.gpu_percent),
            " speed ".dim(),
            self.view.tokens_sparkline.clone().green(),
            " ".into(),
            Self::colored_token_speed(self.view.tokens_per_second),
            " space ".dim(),
            Self::colored_context_percent(self.view.context_remaining_percent),
        ])
    }

    fn line(&self, width: u16) -> Line<'static> {
        if width >= 96 {
            self.wide_line()
        } else {
            self.compact_line()
        }
    }

    fn colored_percent(value: Option<f32>) -> Span<'static> {
        let Some(value) = value else {
            return "n/a".dim();
        };
        let label = format!("{value:.0}%");
        if value >= 85.0 {
            label.red()
        } else if value >= 65.0 {
            label.magenta()
        } else {
            label.green()
        }
    }

    fn colored_token_speed(value: Option<f32>) -> Span<'static> {
        let Some(value) = value else {
            return "warming".dim();
        };
        let label = format!("{value:.1}");
        if value >= 45.0 {
            label.green()
        } else if value >= 20.0 {
            label.cyan()
        } else {
            label.magenta()
        }
    }

    fn colored_context_percent(value: Option<i64>) -> Span<'static> {
        let Some(value) = value else {
            return "n/a".dim();
        };
        let value = value.clamp(0, 100);
        let label = format!("{value}%");
        if value <= 20 {
            label.red()
        } else if value <= 45 {
            label.magenta()
        } else {
            label.green()
        }
    }
}

impl Renderable for StatusRail {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        Paragraph::new(self.line(area.width)).render_ref(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::SPARKLINE_WIDTH;
    use super::StatusRailSample;
    use super::StatusRailTracker;
    use pretty_assertions::assert_eq;

    #[test]
    fn tracker_builds_fixed_width_sparklines() {
        let mut tracker = StatusRailTracker::default();
        for step in 0..32 {
            let view = tracker.observe(StatusRailSample {
                cpu_percent: Some(step as f32),
                memory_percent: Some((step * 3) as f32),
                gpu_percent: Some((step * 2) as f32),
                tokens_per_second: Some((step / 2) as f32),
                context_remaining_percent: Some((100 - step as i64).max(0)),
            });
            assert_eq!(view.cpu_sparkline.chars().count(), SPARKLINE_WIDTH);
            assert_eq!(view.memory_sparkline.chars().count(), SPARKLINE_WIDTH);
            assert_eq!(view.gpu_sparkline.chars().count(), SPARKLINE_WIDTH);
            assert_eq!(view.tokens_sparkline.chars().count(), SPARKLINE_WIDTH);
            assert_eq!(view.context_sparkline.chars().count(), SPARKLINE_WIDTH);
        }
    }
}
