use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::biology::SystemHealthReport;

/// Renders the metabolic health section of the dashboard.
/// Visualizes the system's "Biology" (Expression Rate, Tokens, Throttling).
pub fn render_metabolic_health(f: &mut Frame, area: Rect, health: &SystemHealthReport) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Global Expression Rate
            Constraint::Min(5),    // Specialist Token Buckets
        ])
        .split(area);

    // 1. Global Expression Rate Gauge
    let rate_color = match health.throttle_state {
        crate::biology::ThrottleState::Normal => Color::Green,
        crate::biology::ThrottleState::Metabolic => Color::Yellow,
        crate::biology::ThrottleState::Dormant => Color::Red,
    };

    let rate_gauge = Gauge::default()
        .block(Block::default().title("Metabolic Expression Rate").borders(Borders::ALL))
        .gauge_style(Style::default().fg(rate_color))
        .percent((health.expression_rate * 100.0) as u16)
        .label(format!("{}: {:.2}", health.throttle_state, health.expression_rate));
    
    f.render_widget(rate_gauge, chunks[0]);

    // 2. Specialist Token Status List
    let items: Vec<ListItem> = health.specialist_health.iter().map(|s| {
        let availability = s.token_availability * 100.0;
        let color = if availability > 70.0 { Color::Green } else if availability > 30.0 { Color::Yellow } else { Color::Red };
        
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<20}", s.specialist_id), Style::default().bold()),
            Span::raw(" ["),
            Span::styled(format!("{:>3.0}%", availability), Style::default().fg(color)),
            Span::raw("] "),
            Span::styled(format!("Execs: {}", s.execution_count), Style::default().dim()),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default().title("Specialist Metabolism (Token Buckets)").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, chunks[1]);
}
