use crate::hox_map_schema::{EnzymeGenetics, HoxPermissions};
use anyhow::Result;
use rand::Rng;

pub struct GeneticRecombinator;

impl GeneticRecombinator {
    /// Performs crossover breeding between two specialists.
    /// This creates a new genetic template for a descendant enzyme.
    pub fn breed(parent_a: &EnzymeGenetics, parent_b: &EnzymeGenetics) -> Result<EnzymeGenetics> {
        let mut rng = rand::thread_rng();

        // 1. Category Inheritance (Pick one)
        let category = if rng.gen_bool(0.5) {
            parent_a.category.clone()
        } else {
            parent_b.category.clone()
        };

        // 2. Expression Level Blending (Mean + Mutation)
        let base_expression = (parent_a.expression_level + parent_b.expression_level) / 2.0;
        let mutation: f32 = rng.gen_range(-0.05..0.05);
        let expression_level = (base_expression + mutation).clamp(0.0, 1.0);

        // 3. Permission Crossover (Union of safety, Intersection of power)
        // We favor the more restrictive permissions for the hybrid to ensure safety.
        let permissions = HoxPermissions {
            max_sovereignty_tier: parent_a.permissions.max_sovereignty_tier.min(parent_b.permissions.max_sovereignty_tier),
            allow_network: parent_a.permissions.allow_network && parent_b.permissions.allow_network,
            whitelisted_domains: Self::merge_whitelists(&parent_a.permissions.whitelisted_domains, &parent_b.permissions.whitelisted_domains),
            requires_hitl: parent_a.permissions.requires_hitl || parent_b.permissions.requires_hitl,
        };

        // 4. MCP Tool Fusion (Skill Grafting)
        let mut mcp_tools = parent_a.mcp_tools.clone();
        for tool in &parent_b.mcp_tools {
            if !mcp_tools.iter().any(|t| t.name == tool.name) {
                mcp_tools.push(tool.clone());
            }
        }

        Ok(EnzymeGenetics {
            category,
            expression_level,
            permissions,
            mcp_tools,
        })
    }

    fn merge_whitelists(a: &[String], b: &[String]) -> Vec<String> {
        let mut combined = a.to_vec();
        for domain in b {
            if !combined.contains(domain) {
                combined.push(domain.clone());
            }
        }
        combined
    }
}
