use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

// ── Types ─────────────────────────────────────────────────────────────────────

/// What kind of external service this link connects to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// POST JSON payload to any HTTP endpoint
    Webhook,
    /// Discord incoming webhook (rich embed format)
    Discord,
    /// Slack incoming webhook (Block Kit format)
    Slack,
    /// Notion Integration API — append to a database
    Notion,
    /// GitHub — create an issue or comment
    GitHub,
    /// VS Code / Cursor — push to Language Server Protocol notification
    VsCode,
    /// Custom: user-defined template + HTTP target
    Custom,
}

/// Which federation events trigger this link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event types to trigger on. Empty = all events.
    /// Valid values: "execution_complete", "intent_submitted",
    /// "guild_decomposition", "specialist_update"
    pub event_types: Vec<String>,
    /// Only trigger if the sovereign name matches (empty = any sovereign)
    pub sovereigns: Vec<String>,
    /// Only trigger on specific statuses (empty = any)
    /// Valid: "Success", "Failed", "Partial"
    pub statuses: Vec<String>,
    /// Only trigger if output contains this substring (empty = no filter)
    pub output_contains: Option<String>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            event_types: vec![],
            sovereigns: vec![],
            statuses: vec![],
            output_contains: None,
        }
    }
}

impl EventFilter {
    /// Returns true if this event matches the filter.
    pub fn matches(&self, event: &serde_json::Value) -> bool {
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let sovereign = event.get("specialist").and_then(|v| v.as_str()).unwrap_or("");
        let status = event.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let output = event.get("output_preview").and_then(|v| v.as_str()).unwrap_or("");

        if !self.event_types.is_empty() && !self.event_types.iter().any(|t| t == event_type) {
            return false;
        }
        if !self.sovereigns.is_empty() && !self.sovereigns.iter().any(|s| s.to_lowercase() == sovereign.to_lowercase()) {
            return false;
        }
        if !self.statuses.is_empty() && !self.statuses.iter().any(|s| s == status) {
            return false;
        }
        if let Some(ref contains) = self.output_contains {
            if !output.contains(contains.as_str()) {
                return false;
            }
        }
        true
    }
}

/// A registered integration link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Unique identifier
    pub id: String,
    /// Human-readable name (e.g. "Merlin → Discord research channel")
    pub name: String,
    /// Link type
    pub link_type: LinkType,
    /// Primary target URL (webhook endpoint, Discord webhook URL, etc.)
    pub target_url: String,
    /// Optional API key / token for the target service
    pub api_key: Option<String>,
    /// Notion database ID (only for LinkType::Notion)
    pub notion_database_id: Option<String>,
    /// GitHub repo (e.g. "owner/repo" — only for LinkType::GitHub)
    pub github_repo: Option<String>,
    /// Event filter — which events trigger this link
    pub filter: EventFilter,
    /// Whether this link is active
    pub enabled: bool,
    /// When this link was registered (Unix ms)
    pub created_at: u64,
    /// Delivery statistics
    pub deliveries_sent: u64,
    pub deliveries_failed: u64,
    pub last_delivery_at: Option<u64>,
    pub last_delivery_status: Option<String>,
}

impl Link {
    pub fn new(name: &str, link_type: LinkType, target_url: &str) -> Self {
        Self {
            id: format!("link-{}", now_ms()),
            name: name.to_string(),
            link_type,
            target_url: target_url.to_string(),
            api_key: None,
            notion_database_id: None,
            github_repo: None,
            filter: EventFilter::default(),
            enabled: true,
            created_at: now_ms(),
            deliveries_sent: 0,
            deliveries_failed: 0,
            last_delivery_at: None,
            last_delivery_status: None,
        }
    }
}

pub type LinkRegistry = Arc<RwLock<Vec<Link>>>;

// ── Registry ───────────────────────────────────────────────────────────────────

/// Load links from disk (creates empty registry if file doesn't exist).
pub fn load_links() -> Vec<Link> {
    let paths = crate::workspace::WorkspacePaths::discover();
    let path = paths.links_config();
    if !path.exists() { return vec![]; }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Save links to disk.
pub fn save_links(links: &[Link]) -> anyhow::Result<()> {
    let paths = crate::workspace::WorkspacePaths::discover();
    let path = paths.links_config();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(links)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// ── Dispatcher ────────────────────────────────────────────────────────────────

/// Start the background link dispatcher.
///
/// Subscribes to the federation's specialist_events broadcast channel.
/// For each event, checks all enabled links and dispatches matching ones
/// to their target endpoints via HTTP.
pub fn start_link_dispatcher(
    registry: LinkRegistry,
    mut event_rx: broadcast::Receiver<serde_json::Value>,
) {
    tokio::spawn(async move {
        info!("Link dispatcher started — watching federation events");
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let links = registry.read().await;
                    let matching: Vec<Link> = links.iter()
                        .filter(|l| l.enabled && l.filter.matches(&event))
                        .cloned()
                        .collect();
                    drop(links);

                    for link in matching {
                        let event_clone = event.clone();
                        let registry_clone = registry.clone();
                        tokio::spawn(async move {
                            dispatch_link(&link, &event_clone, registry_clone).await;
                        });
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Link dispatcher lagged by {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Link dispatcher: event channel closed, stopping");
                    break;
                }
            }
        }
    });
}

/// Deliver one event to one link target.
async fn dispatch_link(
    link: &Link,
    event: &serde_json::Value,
    registry: LinkRegistry,
) {
    let payload = format_payload(link, event);
    let result = deliver(link, payload).await;

    // Update delivery statistics in registry
    let mut links = registry.write().await;
    if let Some(l) = links.iter_mut().find(|l| l.id == link.id) {
        l.last_delivery_at = Some(now_ms());
        match result {
            Ok(status) => {
                l.deliveries_sent += 1;
                l.last_delivery_status = Some(format!("HTTP {}", status));
                debug!("Link '{}' delivered (HTTP {})", l.name, status);
            }
            Err(e) => {
                l.deliveries_failed += 1;
                l.last_delivery_status = Some(format!("Error: {}", e));
                warn!("Link '{}' delivery failed: {}", l.name, e);
            }
        }
        // Persist updated stats
        let links_snapshot = links.clone();
        drop(links);
        if let Err(e) = save_links(&links_snapshot) {
            warn!("Failed to persist link stats: {}", e);
        }
        return;
    }
    drop(links);
}

/// Public wrapper for use by the HTTP test endpoint
pub fn format_payload_pub(link: &Link, event: &serde_json::Value) -> serde_json::Value {
    format_payload(link, event)
}

/// Public wrapper for use by the HTTP test endpoint
pub async fn deliver_pub(link: &Link, payload: serde_json::Value) -> anyhow::Result<u16> {
    deliver(link, payload).await
}

/// Format the event payload for the link's target type.
fn format_payload(link: &Link, event: &serde_json::Value) -> serde_json::Value {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("event");
    let sovereign = event.get("specialist").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let status = event.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let output = event.get("output_preview").and_then(|v| v.as_str()).unwrap_or("");
    let intent = event.get("intent").and_then(|v| v.as_str()).unwrap_or("");
    let duration_ms = event.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

    match link.link_type {
        LinkType::Discord => {
            // Discord rich embed format
            let color = match status {
                "Success" => 0x00cc88u32,
                "Failed"  => 0xff4455u32,
                _         => 0xfbbf24u32,
            };
            serde_json::json!({
                "embeds": [{
                    "title": format!("{} — {}", sovereign, event_type.replace('_', " ")),
                    "description": if output.is_empty() { intent } else { output },
                    "color": color,
                    "fields": [
                        { "name": "Sovereign", "value": sovereign, "inline": true },
                        { "name": "Status", "value": if status.is_empty() { "—" } else { status }, "inline": true },
                        { "name": "Duration", "value": format!("{}ms", duration_ms), "inline": true },
                    ],
                    "footer": { "text": "Aaroneous Sovereign Hive" },
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }],
                "username": "Aaroneous",
            })
        }
        LinkType::Slack => {
            // Slack Block Kit format
            let header_emoji = match status {
                "Success" => ":white_check_mark:",
                "Failed"  => ":x:",
                _         => ":zap:",
            };
            serde_json::json!({
                "blocks": [
                    {
                        "type": "header",
                        "text": { "type": "plain_text", "text": format!("{} {} — {}", header_emoji, sovereign, event_type.replace('_', " ")) }
                    },
                    {
                        "type": "section",
                        "text": { "type": "mrkdwn", "text": if output.is_empty() { intent } else { output } }
                    },
                    {
                        "type": "context",
                        "elements": [
                            { "type": "mrkdwn", "text": format!("*Status:* {} | *Duration:* {}ms | Aaroneous", if status.is_empty() { "—" } else { status }, duration_ms) }
                        ]
                    }
                ]
            })
        }
        LinkType::Notion => {
            // Notion API — append a paragraph block to a database page
            serde_json::json!({
                "parent": { "database_id": link.notion_database_id.as_deref().unwrap_or("") },
                "properties": {
                    "Name": { "title": [{ "text": { "content": format!("[{}] {}", sovereign, event_type) } }] },
                    "Status": { "select": { "name": if status.is_empty() { "Event" } else { status } } },
                    "Sovereign": { "select": { "name": sovereign } },
                    "Output": { "rich_text": [{ "text": { "content": &output[..output.len().min(2000)] } }] },
                }
            })
        }
        LinkType::GitHub => {
            // GitHub Issues API body
            serde_json::json!({
                "title": format!("[Aaroneous/{sovereign}] {event_type}"),
                "body": format!(
                    "## Sovereign Event\n\n**Sovereign:** {}\n**Event:** {}\n**Status:** {}\n\n### Output\n\n```\n{}\n```",
                    sovereign, event_type, status, &output[..output.len().min(5000)]
                ),
                "labels": ["aaroneous", sovereign.to_lowercase()],
            })
        }
        LinkType::VsCode => {
            // VS Code LSP window/showMessage format
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "window/showMessage",
                "params": {
                    "type": match status { "Failed" => 1, _ => 3 },
                    "message": format!("[Aaroneous/{sovereign}] {}: {}", event_type, &output[..output.len().min(200)])
                }
            })
        }
        // Generic webhook / custom — raw event with Aaroneous wrapper
        _ => serde_json::json!({
            "source": "aaroneous",
            "version": "2.0",
            "event": event,
            "link_name": link.name,
        }),
    }
}

/// Send the formatted payload to the link's target URL.
async fn deliver(link: &Link, payload: serde_json::Value) -> anyhow::Result<u16> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut req = client.post(&link.target_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "Aaroneous/2.0")
        .json(&payload);

    // Add auth header if API key provided
    if let Some(ref key) = link.api_key {
        req = match link.link_type {
            LinkType::Notion => req.header("Authorization", format!("Bearer {}", key))
                                   .header("Notion-Version", "2022-06-28"),
            LinkType::GitHub => req.header("Authorization", format!("token {}", key)),
            _                => req.header("Authorization", format!("Bearer {}", key)),
        };
    }

    let resp = req.send().await?;
    Ok(resp.status().as_u16())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
