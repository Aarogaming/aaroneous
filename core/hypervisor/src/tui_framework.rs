// Aaroneous Terminal UI Framework
// Ratatui-based dashboard for hive monitoring and management

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    prelude::*,
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem},
    text::{Line, Span},
};
use crate::dashboard::render_metabolic_health;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

/// TUI Application State
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Home,
    Metabolic,
    Specialists,
    SkillTree,
    EventLog,
    Settings,
    Lore,
    SpatialKinetic,
}

/// TUI Event from user interaction
#[derive(Debug)]
pub enum TuiEvent {
    Tick,
    Input(KeyEvent),
    Resize(u16, u16),
    Stop,
}

/// Main TUI Application
pub struct TuiApp {
    pub page: Page,
    pub running: bool,
    pub selected_specialist: Option<String>,
    pub scroll_offset: usize,
    pub system_health: f64, // 0.0-100.0
    pub specialist_count: u32,
    pub total_xp: u32,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            page: Page::Home,
            running: true,
            selected_specialist: None,
            scroll_offset: 0,
            system_health: 85.0,
            specialist_count: 6,
            total_xp: 12500,
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next_page(&mut self) {
        self.page = match self.page {
            Page::Home => Page::Metabolic,
            Page::Metabolic => Page::Specialists,
            Page::Specialists => Page::SkillTree,
            Page::SkillTree => Page::EventLog,
            Page::EventLog => Page::Settings,
            Page::Settings => Page::Lore,
            Page::Lore => Page::SpatialKinetic,
            Page::SpatialKinetic => Page::Home,
        };
        self.scroll_offset = 0;
    }

    pub fn prev_page(&mut self) {
        self.page = match self.page {
            Page::Home => Page::SpatialKinetic,
            Page::Metabolic => Page::Home,
            Page::Specialists => Page::Metabolic,
            Page::SkillTree => Page::Specialists,
            Page::EventLog => Page::SkillTree,
            Page::Settings => Page::EventLog,
            Page::Lore => Page::Settings,
            Page::SpatialKinetic => Page::Lore,
        };
        self.scroll_offset = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }
}

/// Render the home page
pub fn draw_home(f: &mut Frame, area: Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(5),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("⚡ Aaroneous Hive Dashboard")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // System Health
    let health_gauge = Gauge::default()
        .block(Block::default().title("System Health").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(app.system_health as u16);
    f.render_widget(health_gauge, chunks[1]);

    // Stats
    let stats_text = vec![
        Line::from(vec![
            Span::styled("Active Specialists: ", Style::default().bold()),
            Span::raw(app.specialist_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Total XP: ", Style::default().bold()),
            Span::raw(app.total_xp.to_string()),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press 'q' to quit, Tab to navigate", Style::default().dim())),
    ];
    let stats = Paragraph::new(stats_text)
        .block(Block::default().title("System Status").borders(Borders::ALL));
    f.render_widget(stats, chunks[2]);
}

/// Render the specialists page
pub fn draw_specialists(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let specialists = vec![
        "Presenter (UI Designer) - Level 8 - 2,500 XP",
        "Synthesizer (Knowledge) - Level 7 - 2,200 XP",
        "Orchestrator (Leadership) - Level 6 - 1,900 XP",
        "Archivist (Experience) - Level 5 - 1,600 XP",
        "Fabricator (Manufacturing) - Level 4 - 1,200 XP",
        "Sentinel (Security) - Level 3 - 800 XP",
    ];

    let items: Vec<ListItem> = specialists
        .iter()
        .map(|s| ListItem::new(*s))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Active Specialists").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::Blue).bold());

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(0));

    f.render_stateful_widget(list, area, &mut list_state);
}

/// Render the skill tree page
pub fn draw_skill_tree(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let skills = vec![
        "DAG (Level 5) - Task Decomposition",
        "RAG (Level 4) - Knowledge Synthesis",
        "MCP (Level 3) - Tool Integration",
        "API (Level 2) - Federation",
        "Fusion (Level 1) - Skill Combination",
    ];

    let items: Vec<ListItem> = skills
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let prefix = if i == 0 { "⭐ " } else { "  " };
            ListItem::new(format!("{}{}", prefix, s))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Specialist Skills").borders(Borders::ALL))
        .style(Style::default().fg(Color::Green));

    f.render_widget(list, area);
}

/// Render the event log page
pub fn draw_event_log(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let events = vec![
        "[INFO] Presenter leveled up to 8! 🎉",
        "[SKILL] Synthesizer fused DAG + RAG into SuperDAG",
        "[XP] Archivist earned 250 XP from file ingestion",
        "[EVENT] Fabricator breakthrough detected!",
        "[RANK] Orchestrator promoted to Rank 3",
    ];

    let items: Vec<ListItem> = events
        .iter()
        .map(|e| ListItem::new(*e))
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Recent Events").borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(list, area);
}

/// Render the settings page
pub fn draw_settings(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let settings = vec![
        Line::from(Span::styled("⚙️ Settings", Style::default().bold().cyan())),
        Line::from(""),
        Line::from("Log Level: INFO"),
        Line::from("Persistence: Enabled"),
        Line::from("Federation: Ready"),
        Line::from("Tracing: JSON output"),
        Line::from(""),
        Line::from(Span::styled("Up/Down: Scroll", Style::default().dim())),
        Line::from(Span::styled("Tab: Navigate pages", Style::default().dim())),
        Line::from(Span::styled("Q: Quit", Style::default().dim())),
    ];

    let block = Paragraph::new(settings)
        .block(Block::default().title("System Settings").borders(Borders::ALL));

    f.render_widget(block, area);
}

/// Main render function
pub fn draw(f: &mut Frame, app: &TuiApp) {
    let area = f.size();
    
    // Top bar with page title
    let header_area = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: 1,
    };

    let page_title = match app.page {
        Page::Home => "Home",
        Page::Metabolic => "Metabolic Health",
        Page::Specialists => "Specialists",
        Page::SkillTree => "Skill Tree",
        Page::EventLog => "Event Log",
        Page::Settings => "Settings",
        Page::Lore => "Lore",
        Page::SpatialKinetic => "Spatial-Kinetic Engine",
    };

    let header = Paragraph::new(format!("Page: {} | Use Tab to navigate, Q to quit", page_title))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(header, header_area);

    // Content area
    let content_area = Rect {
        x: 0,
        y: 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    match app.page {
        Page::Home => draw_home(f, content_area, app),
        Page::Metabolic => {
            // Mock health report for TUI visualization
            let health = crate::SystemHealthReport {
                global_tokens: 85.0,
                expression_rate: 0.9,
                specialist_count: 3,
                specialist_health: vec![
                    crate::SpecialistHealth {
                        specialist_id: "genesis_architect".to_string(),
                        tokens: 9.0,
                        max_tokens: 10.0,
                        execution_count: 42,
                        token_availability: 0.9,
                    },
                    crate::SpecialistHealth {
                        specialist_id: "soul_forge".to_string(),
                        tokens: 7.5,
                        max_tokens: 10.0,
                        execution_count: 38,
                        token_availability: 0.75,
                    },
                    crate::SpecialistHealth {
                        specialist_id: "rel_omni".to_string(),
                        tokens: 8.2,
                        max_tokens: 10.0,
                        execution_count: 56,
                        token_availability: 0.82,
                    },
                ],
                throttle_state: crate::ThrottleState::Normal,
            };
            render_metabolic_health(f, content_area, &health);
        }
        Page::Specialists => draw_specialists(f, content_area, app),
        Page::SkillTree => draw_skill_tree(f, content_area, app),
        Page::EventLog => draw_event_log(f, content_area, app),
        Page::Settings => draw_settings(f, content_area, app),
        Page::Lore => draw_settings(f, content_area, app),
        Page::SpatialKinetic => draw_settings(f, content_area, app),
    }
}

/// Terminal UI runner
pub struct TuiRunner {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    tx: mpsc::UnboundedSender<TuiEvent>,
    rx: mpsc::UnboundedReceiver<TuiEvent>,
}

impl TuiRunner {
    /// Initialize the TUI
    pub fn new() -> Result<Self, io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self { terminal, tx, rx })
    }

    /// Run the event loop
    pub async fn run(&mut self, mut app: TuiApp) -> Result<(), Box<dyn std::error::Error>> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = std::time::Instant::now();

        let event_tx = self.tx.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(Duration::from_millis(100)).ok() == Some(true) {
                    if let Ok(Event::Key(key)) = event::read() {
                        let _ = event_tx.send(TuiEvent::Input(key));
                    }
                }

                let now = std::time::Instant::now();
                if now.duration_since(last_tick) >= tick_rate {
                    let _ = event_tx.send(TuiEvent::Tick);
                    last_tick = now;
                }
            }
        });

        while app.running {
            self.terminal.draw(|f| draw(f, &app))?;

            if let Ok(event) = tokio::time::timeout(
                Duration::from_millis(100),
                async {
                    self.rx.recv().await
                },
            )
            .await
            {
                if let Some(event) = event {
                    match event {
                        TuiEvent::Input(key) => {
                            match (key.code, key.modifiers) {
                                (KeyCode::Char('q'), KeyModifiers::NONE) => app.quit(),
                                (KeyCode::Tab, KeyModifiers::NONE) => app.next_page(),
                                (KeyCode::BackTab, KeyModifiers::SHIFT) => app.prev_page(),
                                (KeyCode::Up, KeyModifiers::NONE) => app.scroll_up(),
                                (KeyCode::Down, KeyModifiers::NONE) => app.scroll_down(),
                                _ => {}
                            }
                        }
                        TuiEvent::Tick => {
                            // Update app state here
                        }
                        TuiEvent::Stop => app.quit(),
                        _ => {}
                    }
                }
            }
        }

        self.cleanup()?;
        Ok(())
    }

    /// Cleanup TUI
    fn cleanup(&mut self) -> Result<(), io::Error> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = TuiApp::new();
        assert_eq!(app.page, Page::Home);
        assert!(app.running);
        assert_eq!(app.specialist_count, 6);
    }

    #[test]
    fn test_page_navigation() {
        let mut app = TuiApp::new();
        app.next_page();
        assert_eq!(app.page, Page::Metabolic);
        app.next_page();
        assert_eq!(app.page, Page::Specialists);
        app.next_page();
        assert_eq!(app.page, Page::SkillTree);
        app.prev_page();
        assert_eq!(app.page, Page::Specialists);
    }

    #[test]
    fn test_scrolling() {
        let mut app = TuiApp::new();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_down();
        assert_eq!(app.scroll_offset, 1);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0); // Don't go negative
    }

    #[test]
    fn test_quit() {
        let mut app = TuiApp::new();
        assert!(app.running);
        app.quit();
        assert!(!app.running);
    }
}
