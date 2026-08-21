use crate::unified_registry::{EntryMeta, Registry, RegistryConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

/// Registry of links between components — backed by unified Registry.
pub struct LinkRegistry {
    inner: Registry<Link>,
}

impl LinkRegistry {
    pub fn new() -> Self {
        Self {
            inner: Registry::new(RegistryConfig {
                persist_path: Some(std::path::PathBuf::from(DEFAULT_LINKS_PATH)),
                ..Default::default()
            }),
        }
    }

    pub fn with_persist_path(path: &Path) -> Self {
        Self {
            inner: Registry::with_persistence(RegistryConfig {
                persist_path: Some(path.to_path_buf()),
                ..Default::default()
            }),
        }
    }

    /// Add a link. The link's name is used as the key.
    pub fn add(&mut self, link: Link) -> Result<(), String> {
        let id = link.name.clone();
        let meta = EntryMeta::new("1.0.0").with_tags(vec![format!("{:?}", link.link_type)]);
        self.inner.register(id, link, meta)
    }

    /// Get a link by name.
    pub fn get(&self, name: &str) -> Option<Link> {
        self.inner.get(name).map(|e| e.data)
    }

    /// Get a link by name and update last_seen.
    pub fn get_mut(&mut self, name: &str) -> Option<Link> {
        self.inner.get_mut(name).map(|e| e.data)
    }

    /// Find links matching a filter.
    pub fn filter(&self, filter: &EventFilter) -> Vec<Link> {
        self.inner
            .find(|e| {
                filter
                    .link_type
                    .as_ref()
                    .is_none_or(|t| &e.data.link_type == t)
                    && filter.source.as_ref().is_none_or(|s| e.data.name == *s)
                    && filter
                        .target
                        .as_ref()
                        .is_none_or(|t| e.data.target_url == *t)
            })
            .into_iter()
            .map(|e| e.data.clone())
            .collect()
    }

    /// List all link names.
    pub fn list_names(&self) -> Vec<String> {
        self.inner
            .list_ids()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// List all links.
    pub fn list(&self) -> Vec<Link> {
        self.inner
            .list()
            .into_iter()
            .map(|e| e.data.clone())
            .collect()
    }

    /// Remove a link by name.
    pub fn remove(&mut self, name: &str) -> bool {
        self.inner.unregister(name)
    }

    /// Count of links.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Save to the configured persist path.
    pub fn save(&self) -> anyhow::Result<()> {
        let links = self.inner.list();
        let json = serde_json::to_string_pretty(&links)?;
        std::fs::write(DEFAULT_LINKS_PATH, json)?;
        Ok(())
    }
}

impl Default for LinkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Load links from the default JSON file (legacy format compatibility).
pub fn load_links() -> anyhow::Result<LinkRegistry> {
    let path = std::path::Path::new(DEFAULT_LINKS_PATH);
    Ok(LinkRegistry::with_persist_path(path))
}

/// Save links to the default JSON file.
pub fn save_links(registry: &LinkRegistry) -> anyhow::Result<()> {
    registry.save()
}

pub async fn start_link_dispatcher(
    links: Vec<Link>,
    rx: tokio::sync::broadcast::Receiver<serde_json::Value>,
) {
    let _ = links;
    let _ = rx;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut reg = LinkRegistry::new();
        let link = Link::new("test", LinkType::GitHub, "https://github.com/test");
        reg.add(link).unwrap();

        assert_eq!(reg.len(), 1);
        let got = reg.get("test").unwrap();
        assert_eq!(got.target_url, "https://github.com/test");
    }

    #[test]
    fn test_filter() {
        let mut reg = LinkRegistry::new();
        reg.add(Link::new("gh", LinkType::GitHub, "https://github.com"))
            .unwrap();
        reg.add(Link::new("slack", LinkType::Slack, "https://slack.com"))
            .unwrap();

        let gh_links = reg.filter(&EventFilter {
            link_type: Some(LinkType::GitHub),
            ..Default::default()
        });
        assert_eq!(gh_links.len(), 1);
        assert_eq!(gh_links[0].name, "gh");
    }

    #[test]
    fn test_remove() {
        let mut reg = LinkRegistry::new();
        reg.add(Link::new("test", LinkType::Custom, "https://test.com"))
            .unwrap();
        assert!(reg.remove("test"));
        assert_eq!(reg.len(), 0);
    }
}
