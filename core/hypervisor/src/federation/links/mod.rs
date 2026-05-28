use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

pub const DEFAULT_LINKS_PATH: &str = "links_registry.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    Dependency,
    Reference,
    Inheritance,
    Association,
    Discord,
    Slack,
    Notion,
    GitHub,
    VsCode,
    Custom,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: Option<String>,
    pub name: String,
    pub target_url: String,
    pub link_type: LinkType,
    pub api_key: Option<String>,
    pub notion_database_id: Option<String>,
    pub github_repo: Option<String>,
    pub filter: Option<EventFilter>,
    pub enabled: bool,
    pub deliveries_sent: u64,
    pub deliveries_failed: u64,
    pub last_delivery_at: Option<String>,
    pub last_delivery_status: Option<String>,
}

impl Link {
    pub fn new(name: &str, link_type: LinkType, target_url: &str) -> Self {
        Self {
            id: Some(format!("lnk_{}", uuid::Uuid::new_v4())),
            name: name.to_string(),
            target_url: target_url.to_string(),
            link_type,
            api_key: None,
            notion_database_id: None,
            github_repo: None,
            filter: None,
            enabled: true,
            deliveries_sent: 0,
            deliveries_failed: 0,
            last_delivery_at: None,
            last_delivery_status: None,
        }
    }
}

/// Filter for querying links
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    pub link_type: Option<LinkType>,
    pub source: Option<String>,
    pub target: Option<String>,
}

/// Registry of links between components
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkRegistry {
    links: Vec<Link>,
}

impl Deref for LinkRegistry {
    type Target = Vec<Link>;
    fn deref(&self) -> &Vec<Link> {
        &self.links
    }
}

impl DerefMut for LinkRegistry {
    fn deref_mut(&mut self) -> &mut Vec<Link> {
        &mut self.links
    }
}

impl LinkRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, link: Link) {
        self.links.push(link);
    }

    pub fn get(&self, source: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.name == source).collect()
    }

    pub fn filter(&self, filter: &EventFilter) -> Vec<&Link> {
        self.links.iter().filter(|link| {
            filter.link_type.as_ref().map_or(true, |t| &link.link_type == t)
                && filter.source.as_ref().map_or(true, |s| link.name == *s)
                && filter.target.as_ref().map_or(true, |t| link.target_url == *t)
        }).collect()
    }

    pub fn len(&self) -> usize {
        self.links.len()
    }
}

/// Load links from the default JSON file
pub fn load_links() -> anyhow::Result<LinkRegistry> {
    let content = std::fs::read_to_string(DEFAULT_LINKS_PATH)?;
    let registry: LinkRegistry = serde_json::from_str(&content)?;
    Ok(registry)
}

/// Save links to the default JSON file
pub fn save_links(registry: &LinkRegistry) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(registry)?;
    std::fs::write(DEFAULT_LINKS_PATH, content)?;
    Ok(())
}

pub async fn start_link_dispatcher(links: LinkRegistry, rx: tokio::sync::broadcast::Receiver<serde_json::Value>) {
    let _ = links;
    let _ = rx;
}
