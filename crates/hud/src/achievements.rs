// core/hypervisor/src/hud/achievements.rs
//! Achievement and Training Mastery System.
//! Rewards proper system usage, teaches workflows through progressive milestones,
//! and awards XP when actions are performed repeatedly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    Foundations,
    RoutinesAndEmulation,
    ModelCrafting,
    SkillConstellation,
    SpeedAndReflex,
}

impl AchievementCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Foundations => "?? Foundations",
            Self::RoutinesAndEmulation => "?? Routines & Emulation",
            Self::ModelCrafting => "? Model Crafting",
            Self::SkillConstellation => "?? Constellation Skills",
            Self::SpeedAndReflex => "? Speed & Reflex",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: AchievementCategory,
    pub current_progress: u64,
    pub target_progress: u64,
    pub is_unlocked: bool,
    pub xp_reward: u64,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementManager {
    pub achievements: Vec<Achievement>,
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self {
            achievements: vec![
                // 1. Foundations
                Achievement {
                    id: "first_orbit".to_string(),
                    title: "First Orbit".to_string(),
                    description: "Explore the 3D Cosmos map and inspect a star node.".to_string(),
                    category: AchievementCategory::Foundations,
                    current_progress: 1,
                    target_progress: 1,
                    is_unlocked: true,
                    xp_reward: 50,
                    icon: "??".to_string(),
                },
                Achievement {
                    id: "keyboard_virtuoso".to_string(),
                    title: "Keyboard Virtuoso".to_string(),
                    description: "Use shortcuts or the Action Palette (Ctrl+K) 5 times.".to_string(),
                    category: AchievementCategory::Foundations,
                    current_progress: 2,
                    target_progress: 5,
                    is_unlocked: false,
                    xp_reward: 100,
                    icon: "??".to_string(),
                },
                // 2. Routines & Emulation
                Achievement {
                    id: "routine_architect".to_string(),
                    title: "Routine Architect".to_string(),
                    description: "Create and save an automated action routine.".to_string(),
                    category: AchievementCategory::RoutinesAndEmulation,
                    current_progress: 1,
                    target_progress: 1,
                    is_unlocked: true,
                    xp_reward: 150,
                    icon: "???".to_string(),
                },
                Achievement {
                    id: "ghost_in_the_machine".to_string(),
                    title: "Natural Emulation".to_string(),
                    description: "Run 5 action steps through the transparent HUD overlay.".to_string(),
                    category: AchievementCategory::RoutinesAndEmulation,
                    current_progress: 3,
                    target_progress: 5,
                    is_unlocked: false,
                    xp_reward: 200,
                    icon: "??".to_string(),
                },
                Achievement {
                    id: "autopilot_sentinel".to_string(),
                    title: "Auto-Pilot Sentinel".to_string(),
                    description: "Engage Auto-Pilot and execute 50 automated actions.".to_string(),
                    category: AchievementCategory::RoutinesAndEmulation,
                    current_progress: 12,
                    target_progress: 50,
                    is_unlocked: false,
                    xp_reward: 350,
                    icon: "??".to_string(),
                },
                // 3. Model Crafting
                Achievement {
                    id: "apprentice_blacksmith".to_string(),
                    title: "Cartridge Smith".to_string(),
                    description: "Build your first verified offline .si model package.".to_string(),
                    category: AchievementCategory::ModelCrafting,
                    current_progress: 1,
                    target_progress: 1,
                    is_unlocked: true,
                    xp_reward: 250,
                    icon: "?".to_string(),
                },
                Achievement {
                    id: "grand_foundry".to_string(),
                    title: "Master Foundry".to_string(),
                    description: "Synthesize 3 distinct model packages across different roles.".to_string(),
                    category: AchievementCategory::ModelCrafting,
                    current_progress: 1,
                    target_progress: 3,
                    is_unlocked: false,
                    xp_reward: 500,
                    icon: "??".to_string(),
                },
                // 4. Constellation Skills
                Achievement {
                    id: "stellar_initiate".to_string(),
                    title: "Stellar Initiate".to_string(),
                    description: "Unlock your first perk node in the 3D Constellation Skill Tree.".to_string(),
                    category: AchievementCategory::SkillConstellation,
                    current_progress: 1,
                    target_progress: 1,
                    is_unlocked: true,
                    xp_reward: 150,
                    icon: "?".to_string(),
                },
                Achievement {
                    id: "constellation_master".to_string(),
                    title: "Constellation Master".to_string(),
                    description: "Unlock 6 perk nodes across any skill clusters.".to_string(),
                    category: AchievementCategory::SkillConstellation,
                    current_progress: 2,
                    target_progress: 6,
                    is_unlocked: false,
                    xp_reward: 450,
                    icon: "??".to_string(),
                },
                // 5. Speed & Reflex
                Achievement {
                    id: "sub_millisecond".to_string(),
                    title: "Sub-Millisecond Reflex".to_string(),
                    description: "Execute a macro or routine with under 1ms response latency.".to_string(),
                    category: AchievementCategory::SpeedAndReflex,
                    current_progress: 1,
                    target_progress: 1,
                    is_unlocked: true,
                    xp_reward: 300,
                    icon: "?".to_string(),
                },
            ],
        }
    }
}

impl AchievementManager {
    /// Records progress towards an achievement by ID, returns unlocked achievement if freshly achieved
    pub fn record_progress(&mut self, id: &str, amount: u64) -> Option<Achievement> {
        if let Some(ach) = self.achievements.iter_mut().find(|a| a.id == id) {
            if !ach.is_unlocked {
                ach.current_progress = (ach.current_progress + amount).min(ach.target_progress);
                if ach.current_progress >= ach.target_progress {
                    ach.is_unlocked = true;
                    return Some(ach.clone());
                }
            }
        }
        None
    }

    pub fn unlocked_count(&self) -> usize {
        self.achievements.iter().filter(|a| a.is_unlocked).count()
    }

    pub fn total_count(&self) -> usize {
        self.achievements.len()
    }
}
