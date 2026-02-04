use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;

use super::system_stats::SystemStatsSnapshot;
use crate::render::renderable::Renderable;

const LANDING_OPTIONS: [&str; 5] = [
    "Improve something that already exists",
    "Fix an error or failing test",
    "Understand how something works",
    "Just explore safely",
    "Build something from scratch",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HomePanelIntent {
    ImproveExisting,
    FixError,
    UnderstandSystem,
    ExploreSafely,
    BuildFromScratch,
}

impl HomePanelIntent {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ImproveExisting => "Improve something that already exists",
            Self::FixError => "Fix an error or failing test",
            Self::UnderstandSystem => "Understand how something works",
            Self::ExploreSafely => "Just explore safely",
            Self::BuildFromScratch => "Build something from scratch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HomePanelScreen {
    Landing,
    IntentForm,
    Journey,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct HomePanelInsights {
    pub(crate) tokens_per_second: Option<f32>,
    pub(crate) total_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) context_remaining_percent: Option<i64>,
}

pub(crate) struct HomePanelContext {
    pub(crate) project_name: String,
    pub(crate) model: String,
    pub(crate) safety_tier: String,
    pub(crate) approvals: String,
    pub(crate) sandbox: String,
}

#[derive(Clone, Debug)]
pub(crate) struct HomePanelState {
    pub(crate) screen: HomePanelScreen,
    pub(crate) selected_option: usize,
    pub(crate) show_step_help: bool,
}

impl HomePanelState {
    pub(crate) fn new() -> Self {
        Self {
            screen: HomePanelScreen::Landing,
            selected_option: 0,
            show_step_help: false,
        }
    }

    pub(crate) fn move_up(&mut self) {
        if self.selected_option == 0 {
            self.selected_option = LANDING_OPTIONS.len().saturating_sub(1);
        } else {
            self.selected_option -= 1;
        }
    }

    pub(crate) fn move_down(&mut self) {
        self.selected_option = (self.selected_option + 1) % LANDING_OPTIONS.len();
    }

    pub(crate) fn select_option_by_number(&mut self, number: usize) -> bool {
        if (1..=LANDING_OPTIONS.len()).contains(&number) {
            self.selected_option = number - 1;
            return true;
        }

        false
    }

    pub(crate) fn enter(&mut self) {
        match self.screen {
            HomePanelScreen::Landing => self.screen = HomePanelScreen::IntentForm,
            HomePanelScreen::IntentForm => self.screen = HomePanelScreen::Journey,
            HomePanelScreen::Journey => {}
        }
    }

    pub(crate) fn back(&mut self) {
        match self.screen {
            HomePanelScreen::Landing => {}
            HomePanelScreen::IntentForm => self.screen = HomePanelScreen::Landing,
            HomePanelScreen::Journey => self.screen = HomePanelScreen::IntentForm,
        }
    }

    pub(crate) fn selected_intent(&self) -> HomePanelIntent {
        match self.selected_option {
            0 => HomePanelIntent::ImproveExisting,
            1 => HomePanelIntent::FixError,
            2 => HomePanelIntent::UnderstandSystem,
            3 => HomePanelIntent::ExploreSafely,
            _ => HomePanelIntent::BuildFromScratch,
        }
    }
}

pub(crate) struct HomePanel {
    project_name: String,
    model: String,
    safety_tier: String,
    approvals: String,
    sandbox: String,
    system_stats: SystemStatsSnapshot,
    insights: HomePanelInsights,
    state: HomePanelState,
}

impl HomePanel {
    pub(crate) fn new(
        context: HomePanelContext,
        system_stats: SystemStatsSnapshot,
        insights: HomePanelInsights,
        state: HomePanelState,
    ) -> Self {
        Self {
            project_name: context.project_name,
            model: context.model,
            safety_tier: context.safety_tier,
            approvals: context.approvals,
            sandbox: context.sandbox,
            system_stats,
            insights,
            state,
        }
    }

    fn safety_line(&self) -> String {
        format!(
            "{} | {} / {}",
            self.safety_tier, self.approvals, self.sandbox
        )
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

    fn colored_token_speed(value: Option<f32>) -> Span<'static> {
        let Some(value) = value else {
            return "warming up".dim();
        };
        let value = value.max(0.0);
        let label = format!("{value:.1} tok/s");
        if value >= 45.0 {
            label.green()
        } else if value >= 20.0 {
            label.cyan()
        } else {
            label.magenta()
        }
    }

    fn quick_status_line(&self) -> Line<'static> {
        Line::from(vec![
            "Live now ".bold(),
            "CPU ".dim(),
            Self::colored_percent(self.system_stats.cpu_percent),
            "  MEM ".dim(),
            Self::colored_percent(self.system_stats.memory_percent),
            "  GPU ".dim(),
            Self::colored_percent(self.system_stats.gpu_percent),
            "  speed ".dim(),
            Self::colored_token_speed(self.insights.tokens_per_second),
            "  context ".dim(),
            Self::colored_context_percent(self.insights.context_remaining_percent),
        ])
    }

    fn logo_lines() -> [Line<'static>; 9] {
        [
            "      .-''''-.".cyan().into(),
            "    /  .--.  \\".cyan().into(),
            "   |  /    \\  |".cyan().into(),
            "   | |  ()  | |".cyan().into(),
            "   |  \\____/  |".cyan().into(),
            "    \\  '--'  /".cyan().into(),
            "      '-.__.-'".cyan().into(),
            "".into(),
            Line::from(vec![
                "A-Eye".bold().cyan(),
                "  interactive home".bold().dim(),
            ]),
        ]
    }

    fn selected_intent_summary(&self) -> &'static str {
        match self.state.selected_intent() {
            HomePanelIntent::ImproveExisting => {
                "Great for cleanup and confidence-building improvements."
            }
            HomePanelIntent::FixError => {
                "We will prioritize root-cause checks and safe verification."
            }
            HomePanelIntent::UnderstandSystem => {
                "You get a plain-language architecture walk-through first."
            }
            HomePanelIntent::ExploreSafely => {
                "Low-risk discovery mode with strict safety boundaries."
            }
            HomePanelIntent::BuildFromScratch => {
                "We start with requirements, then scaffold a safe implementation plan."
            }
        }
    }

    fn compact_landing_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(vec!["A-Eye ".bold().cyan(), "interactive home".dim()]),
            "Welcome to A-Eye".bold().into(),
            Line::from("Intent-driven guidance for planners and builders.".dim()),
            "".into(),
            Line::from(format!("Project detected: {}", self.project_name)),
            Line::from(format!("Safety: {}", self.safety_line())),
            "".into(),
            "What would you like to do today?".bold().into(),
        ];
        for (idx, option) in LANDING_OPTIONS.iter().enumerate() {
            if idx == self.state.selected_option {
                lines.push(Line::from(format!("▶ [{}] {option}", idx + 1)).cyan());
            } else {
                lines.push(Line::from(format!("  [{}] {option}", idx + 1)));
            }
        }
        lines.push("".into());
        lines.push(Line::from(vec![
            "Selected: ".dim(),
            self.state.selected_intent().label().cyan(),
        ]));
        lines.push(Line::from(self.selected_intent_summary()).dim());
        lines.push("".into());
        lines.push(self.quick_status_line());
        lines.push(Line::from(
            "Live stats stay pinned below (you can ignore them).".dim(),
        ));
        lines.push("".into());
        lines.push(
            "[ 1-5 ] Quick pick   [ ↑ ↓ ] Move   [ Enter ] Continue"
                .dim()
                .into(),
        );
        lines
    }

    fn compact_intent_form_lines(&self) -> Vec<Line<'static>> {
        let (intent, why, care, scope) = self.intent_sample();
        let lines = vec![
            Line::from(vec!["A-Eye ".bold().cyan(), "intent capture".dim()]),
            "Tell me more".bold().into(),
            "".into(),
            Line::from(vec!["Intent: ".dim(), intent.into()]),
            Line::from(vec!["Why: ".dim(), why.into()]),
            Line::from(vec!["Carefulness: ".dim(), care.into()]),
            Line::from(vec!["Where to look (optional): ".dim(), scope.into()]),
            "".into(),
            "Nothing happens without your OK.".dim().into(),
            "".into(),
            self.quick_status_line(),
            Line::from("Live stats stay pinned below (you can ignore them).".dim()),
            "".into(),
            "[ Enter ] Continue   [ Esc ] Back".dim().into(),
        ];
        lines
    }

    fn compact_journey_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(vec!["A-Eye ".bold().cyan(), "guided journey".dim()]),
            "Your Change Journey".bold().into(),
            Line::from(vec!["Current: ".dim(), "🧩 Planning a safe change".cyan()]),
            Line::from(format!(
                "Selected intent: {}",
                self.state.selected_intent().label()
            )),
            "".into(),
            "🧠 Idea  ->  🗺 Understand  ->  🧩 Plan".into(),
            "🛠 Preview  ->  🔍 Verify  ->  🎓 Learn".into(),
            "".into(),
            Line::from(vec![
                "Intent: ".dim(),
                self.state.selected_intent().label().into(),
            ]),
            Line::from(self.selected_intent_summary()).dim(),
            "".into(),
            self.quick_status_line(),
            Line::from("Live stats stay pinned below (you can ignore them).".dim()),
        ];
        if self.state.show_step_help {
            lines.push("".into());
            lines.push(
                "Why this step matters: confirm plan quality before any edits."
                    .dim()
                    .into(),
            );
        }
        lines.push("".into());
        lines.push(
            "[ Enter ] Continue   [ ← ] Back   [ ? ] Why this step"
                .dim()
                .into(),
        );
        lines
    }

    fn landing_lines(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Self::logo_lines().to_vec();
        lines.push("".into());
        lines.push("Welcome to A-Eye".bold().into());
        lines.push("".into());
        lines.push(Line::from(vec![
            "I help you turn ideas ".into(),
            "into".magenta(),
            " a clear plan (and safe changes when needed).".into(),
        ]));
        lines.push(Line::from(vec![
            "No".magenta(),
            " special jargon required. ".into(),
            "No".magenta(),
            " surprises.".into(),
        ]));
        lines.push("".into());
        lines.push(Line::from(format!(
            "Project detected: {}",
            self.project_name
        )));
        lines.push("Last check: just now ✓".green().into());
        lines.push("".into());
        lines.push(Line::from(format!(
            "Safety: {} | model: {}",
            self.safety_line(),
            self.model
        )));
        lines.push("".into());
        lines.push("What would you like to do today?".bold().into());
        lines.push("".into());
        for (idx, option) in LANDING_OPTIONS.iter().enumerate() {
            if idx == self.state.selected_option {
                lines.push(Line::from(format!("▶ [{}] {option}", idx + 1)).cyan());
            } else {
                lines.push(Line::from(format!("  [{}] {option}", idx + 1)));
            }
        }
        lines.push("".into());
        lines.push(Line::from(vec![
            "Selected path: ".dim(),
            self.state.selected_intent().label().cyan(),
        ]));
        lines.push(Line::from(self.selected_intent_summary()).dim());
        lines.push("".into());
        lines.push(self.quick_status_line());
        lines.push(Line::from("Detailed live stats stay pinned below.".dim()));
        lines.push("".into());
        lines.push(
            "[ 1-5 ] Quick pick   [ ↑ ↓ ] Move   [ Enter ] Continue   [ Q ] Exit"
                .dim()
                .into(),
        );
        lines
    }

    fn intent_sample(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self.state.selected_intent() {
            HomePanelIntent::ImproveExisting => (
                "Improve checkout reliability for mobile users",
                "Checkout retries reduce temporary network failures",
                "Very careful",
                "[ src/**, tests/** ]",
            ),
            HomePanelIntent::FixError => (
                "Fix timeout errors in payment service",
                "Users see intermittent failures during peak load",
                "Very careful",
                "[ src/**, tests/** ]",
            ),
            HomePanelIntent::UnderstandSystem => (
                "Help me understand the payment flow",
                "I want confidence before changing anything",
                "Balanced",
                "[ src/**, docs/** ]",
            ),
            HomePanelIntent::ExploreSafely => (
                "Show me safe places to explore",
                "I am learning this project",
                "Very careful",
                "[ docs/**, tests/** ]",
            ),
            HomePanelIntent::BuildFromScratch => (
                "Build a task-tracker app from scratch",
                "I want a complete but beginner-friendly project setup",
                "Balanced",
                "[ src/**, tests/**, docs/** ]",
            ),
        }
    }

    fn intent_form_lines(&self) -> Vec<Line<'static>> {
        let (intent, why, care, scope) = self.intent_sample();
        let mut lines: Vec<Line<'static>> = Self::logo_lines().to_vec();
        lines.push("".into());
        lines.push("Tell me more".bold().into());
        lines.push("".into());
        lines.push("What are you trying to do?".into());
        lines.push(Line::from(vec!["✎ ".dim(), intent.into()]));
        lines.push("".into());
        lines.push("Why does this matter? (optional)".into());
        lines.push(Line::from(vec!["✎ ".dim(), why.into()]));
        lines.push("".into());
        lines.push("How careful should we be?".into());
        lines.push(Line::from(format!("(●) {care}   ( ) Balanced   ( ) Fast")));
        lines.push("".into());
        lines.push("Anything I should look at? (optional)".into());
        lines.push(Line::from(scope).dim());
        lines.push("".into());
        lines.push("I will not do anything yet.".dim().into());
        lines.push("".into());
        lines.push(self.quick_status_line());
        lines.push(Line::from(
            "Live stats stay pinned below (you can ignore them).".dim(),
        ));
        lines.push("".into());
        lines.push("[ Enter ] Continue   [ Esc ] Cancel".dim().into());
        lines
    }

    fn journey_lines(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Self::logo_lines().to_vec();
        lines.push("".into());
        lines.push("Your Change Journey".bold().into());
        lines.push(Line::from(format!(
            "Selected intent: {}",
            self.state.selected_intent().label()
        )));
        lines.push("".into());
        lines.push("  🧠  Idea".into());
        lines.push("   │".dim().into());
        lines.push("  ✅  🗺  Understanding the system".green().into());
        lines.push("   │".dim().into());
        lines.push("  ▶  🧩  Planning a safe change".cyan().into());
        lines.push("   │".dim().into());
        lines.push("     🛠  Previewing the change".into());
        lines.push("   │".dim().into());
        lines.push("     🔍  Verifying it works".into());
        lines.push("   │".dim().into());
        lines.push("     🎓  What you learned".into());
        lines.push("".into());
        lines.push("You are here: 🧩 Planning a safe change".bold().into());
        lines.push("".into());
        lines.push(self.quick_status_line());
        lines.push(Line::from("Detailed live stats stay pinned below.".dim()));
        if self.state.show_step_help {
            lines.push("".into());
            lines.push(
                "Why this step matters: we draft a plan and wait for approval before edits."
                    .dim()
                    .into(),
            );
        }
        lines.push("".into());
        lines.push(
            "[ Enter ] Continue   [ ← ] Go back   [ ? ] Why this step matters"
                .dim()
                .into(),
        );
        lines
    }

    fn lines(&self) -> Vec<Line<'static>> {
        match self.state.screen {
            HomePanelScreen::Landing => self.landing_lines(),
            HomePanelScreen::IntentForm => self.intent_form_lines(),
            HomePanelScreen::Journey => self.journey_lines(),
        }
    }

    fn compact_lines(&self) -> Vec<Line<'static>> {
        match self.state.screen {
            HomePanelScreen::Landing => self.compact_landing_lines(),
            HomePanelScreen::IntentForm => self.compact_intent_form_lines(),
            HomePanelScreen::Journey => self.compact_journey_lines(),
        }
    }
}

impl Renderable for HomePanel {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let inner_width = area.width.saturating_sub(2);
        let full_lines = self.lines();
        let full_height =
            (Paragraph::new(full_lines.clone()).line_count(inner_width) as u16).saturating_add(2);
        let use_compact = full_height > area.height;
        let lines = if use_compact {
            self.compact_lines()
        } else {
            full_lines
        };

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(ratatui::style::Style::new().cyan())
                    .title(" A-Eye "),
            )
            .render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        let inner_width = width.saturating_sub(2);
        let full_height =
            (Paragraph::new(self.lines()).line_count(inner_width) as u16).saturating_add(2);
        let compact_height =
            (Paragraph::new(self.compact_lines()).line_count(inner_width) as u16).saturating_add(2);
        compact_height.min(full_height)
    }
}
