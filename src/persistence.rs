// Aaroneous Persistence Layer - SQLite Integration
// Handles saving/loading specialists, skills, constellations, and event history

use rusqlite::{Connection, Result as SqlResult, params, OptionalExtension};
use serde_json::json;
use crate::skill_system::Skill;
use crate::genetics::SpecialistGenome;
use crate::self_digestion::SpecialistSoul;

/// Persistence manager for the Aaroneous hive
pub struct PersistenceManager {
    db: Connection,
}

impl PersistenceManager {
    /// Initialize persistence layer with SQLite database
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let db = Connection::open(db_path)?;
        
        // Enable foreign keys
        db.execute("PRAGMA foreign_keys = ON", [])?;
        
        let manager = PersistenceManager { db };
        manager.init_schema()?;
        Ok(manager)
    }

    /// Create all necessary tables
    fn init_schema(&self) -> SqlResult<()> {
        self.db.execute_batch(
            "
            -- Specialists table
            CREATE TABLE IF NOT EXISTS specialists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                archetype TEXT NOT NULL,
                birth_timestamp INTEGER NOT NULL,
                generation_level INTEGER NOT NULL,
                xp INTEGER NOT NULL,
                xp_total INTEGER NOT NULL,
                current_level INTEGER NOT NULL,
                rank INTEGER NOT NULL,
                active BOOLEAN NOT NULL DEFAULT 1,
                genetics_json TEXT NOT NULL,
                soul_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Skills table
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                specialist_id TEXT NOT NULL,
                skill_type TEXT NOT NULL,
                origin TEXT NOT NULL,
                level INTEGER NOT NULL,
                power_score REAL NOT NULL,
                success_rate REAL NOT NULL,
                breakthrough_eligible BOOLEAN NOT NULL,
                breakthrough_achieved BOOLEAN NOT NULL,
                awoken BOOLEAN NOT NULL,
                evolved_form_json TEXT,
                skill_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
            );

            -- Skill fusion history
            CREATE TABLE IF NOT EXISTS skill_fusions (
                id TEXT PRIMARY KEY,
                specialist_id TEXT NOT NULL,
                parent_skills TEXT NOT NULL,
                result_skill_json TEXT NOT NULL,
                fusion_quality REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
            );

            -- Constellation nodes
            CREATE TABLE IF NOT EXISTS constellation_nodes (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                label TEXT NOT NULL,
                description TEXT,
                position_x REAL NOT NULL,
                position_y REAL NOT NULL,
                position_z REAL NOT NULL,
                weight REAL NOT NULL,
                specialist_id TEXT,
                timestamp INTEGER NOT NULL,
                node_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
            );

            -- Constellation edges (relationships between nodes)
            CREATE TABLE IF NOT EXISTS constellation_edges (
                id TEXT PRIMARY KEY,
                source_node_id TEXT NOT NULL,
                target_node_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                weight REAL NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_node_id) REFERENCES constellation_nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (target_node_id) REFERENCES constellation_nodes(id) ON DELETE CASCADE
            );

            -- Event history
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                specialist_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                xp_gained INTEGER NOT NULL,
                quality_score REAL NOT NULL,
                description TEXT,
                data_json TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
            );

            -- Rank progression history
            CREATE TABLE IF NOT EXISTS rank_progressions (
                id TEXT PRIMARY KEY,
                specialist_id TEXT NOT NULL,
                from_rank INTEGER NOT NULL,
                to_rank INTEGER NOT NULL,
                milestone_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
            );

             -- Data ingestion history
             CREATE TABLE IF NOT EXISTS ingestion_records (
                 id TEXT PRIMARY KEY,
                 specialist_id TEXT NOT NULL,
                 file_path TEXT NOT NULL,
                 file_format TEXT NOT NULL,
                 file_size INTEGER NOT NULL,
                 checksum TEXT NOT NULL,
                 domain TEXT NOT NULL,
                 xp_generated INTEGER NOT NULL,
                 quality_score REAL NOT NULL,
                 timestamp INTEGER NOT NULL,
                 created_at TEXT NOT NULL,
                 FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
             );

             -- Specialist Memory: Lessons, strategies, decisions
             CREATE TABLE IF NOT EXISTS memory_entries (
                 id TEXT PRIMARY KEY,
                 specialist_id TEXT NOT NULL,
                 memory_type TEXT NOT NULL,
                 title TEXT NOT NULL,
                 description TEXT NOT NULL,
                 context TEXT,
                 confidence TEXT NOT NULL,
                 relevance_score REAL NOT NULL DEFAULT 1.0,
                 usage_count INTEGER NOT NULL DEFAULT 0,
                 tags TEXT,
                 related_memories TEXT,
                 source TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
             );

             -- Decision records with outcomes
             CREATE TABLE IF NOT EXISTS decision_records (
                 id TEXT PRIMARY KEY,
                 specialist_id TEXT NOT NULL,
                 decision TEXT NOT NULL,
                 reasoning TEXT NOT NULL,
                 alternatives_considered TEXT,
                 outcome_success BOOLEAN,
                 outcome_description TEXT,
                 outcome_recorded_at TEXT,
                 confidence_before TEXT NOT NULL,
                 confidence_after TEXT,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
             );

             -- Strategies for task types
             CREATE TABLE IF NOT EXISTS strategies (
                 id TEXT PRIMARY KEY,
                 specialist_id TEXT NOT NULL,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL,
                 steps TEXT NOT NULL,
                 effectiveness_score REAL NOT NULL,
                 success_count INTEGER NOT NULL DEFAULT 0,
                 failure_count INTEGER NOT NULL DEFAULT 0,
                 applicable_to TEXT,
                 prerequisites TEXT,
                 created_at TEXT NOT NULL,
                 last_used TEXT NOT NULL,
                 FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
             );

             -- Goals the specialist is pursuing
             CREATE TABLE IF NOT EXISTS goals (
                 id TEXT PRIMARY KEY,
                 specialist_id TEXT NOT NULL,
                 objective TEXT NOT NULL,
                 reason TEXT NOT NULL,
                 status TEXT NOT NULL,
                 priority INTEGER NOT NULL,
                 created_at TEXT NOT NULL,
                 target_completion TEXT,
                 completed_at TEXT,
                 progress_percentage INTEGER NOT NULL DEFAULT 0,
                 blockers TEXT,
                 milestones TEXT,
                 FOREIGN KEY (specialist_id) REFERENCES specialists(id) ON DELETE CASCADE
             );

             -- Indexes for performance
             CREATE INDEX IF NOT EXISTS idx_specialists_rank ON specialists(rank);
             CREATE INDEX IF NOT EXISTS idx_specialists_updated ON specialists(updated_at);
             CREATE INDEX IF NOT EXISTS idx_skills_specialist ON skills(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_skills_type ON skills(skill_type);
             CREATE INDEX IF NOT EXISTS idx_events_specialist ON events(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
             CREATE INDEX IF NOT EXISTS idx_constellation_specialist ON constellation_nodes(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_ingestion_specialist ON ingestion_records(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_memory_specialist ON memory_entries(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_entries(memory_type);
             CREATE INDEX IF NOT EXISTS idx_decision_specialist ON decision_records(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_strategy_specialist ON strategies(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_goal_specialist ON goals(specialist_id);
             CREATE INDEX IF NOT EXISTS idx_goal_status ON goals(status);
             "
        )?;
        Ok(())
    }

    /// Save a specialist to the database
    pub fn save_specialist(
        &self,
        specialist_id: &str,
        name: &str,
        archetype: &str,
        birth_timestamp: i64,
        generation_level: u32,
        xp: u32,
        xp_total: u32,
        current_level: u32,
        rank: u32,
        genome: &SpecialistGenome,
        soul: &SpecialistSoul,
    ) -> SqlResult<()> {
        let genetics_json = match serde_json::to_string(genome) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize specialist genome: {}", e);
                return Err(rusqlite::Error::InvalidQuery);
            }
        };
        let soul_json = match serde_json::to_string(soul) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize specialist soul: {}", e);
                return Err(rusqlite::Error::InvalidQuery);
            }
        };
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO specialists 
             (id, name, archetype, birth_timestamp, generation_level, xp, xp_total, 
              current_level, rank, genetics_json, soul_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                specialist_id, name, archetype, birth_timestamp, generation_level,
                xp, xp_total, current_level, rank, genetics_json, soul_json, now, now
            ],
        )?;
        Ok(())
    }

    /// Load a specialist from the database
    pub fn load_specialist(&self, specialist_id: &str) -> SqlResult<Option<SpecialistRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, name, archetype, birth_timestamp, generation_level, xp, xp_total,
                    current_level, rank, genetics_json, soul_json, created_at, updated_at
             FROM specialists WHERE id = ?1"
        )?;

        let specialist = stmt.query_row(params![specialist_id], |row| {
            Ok(SpecialistRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                archetype: row.get(2)?,
                birth_timestamp: row.get(3)?,
                generation_level: row.get(4)?,
                xp: row.get(5)?,
                xp_total: row.get(6)?,
                current_level: row.get(7)?,
                rank: row.get(8)?,
                genetics_json: row.get(9)?,
                soul_json: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        }).optional()?;

        Ok(specialist)
    }

    /// List all specialists
    pub fn list_specialists(&self) -> SqlResult<Vec<SpecialistRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, name, archetype, birth_timestamp, generation_level, xp, xp_total,
                    current_level, rank, genetics_json, soul_json, created_at, updated_at
             FROM specialists ORDER BY updated_at DESC"
        )?;

        let specialists = stmt.query_map([], |row| {
            Ok(SpecialistRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                archetype: row.get(2)?,
                birth_timestamp: row.get(3)?,
                generation_level: row.get(4)?,
                xp: row.get(5)?,
                xp_total: row.get(6)?,
                current_level: row.get(7)?,
                rank: row.get(8)?,
                genetics_json: row.get(9)?,
                soul_json: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(specialists)
    }

    /// Save a skill to the database
    pub fn save_skill(
        &self,
        specialist_id: &str,
        skill_id: &str,
        skill: &Skill,
    ) -> SqlResult<()> {
        let skill_json = match serde_json::to_string(skill) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize skill {}: {}", skill_id, e);
                return Err(rusqlite::Error::InvalidQuery);
            }
        };
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO skills 
             (id, specialist_id, skill_type, origin, level, power_score, success_rate,
              breakthrough_eligible, breakthrough_achieved, awoken, skill_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                skill_id, specialist_id, format!("{:?}", skill.skill_type),
                format!("{:?}", skill.origin), skill.level, skill.success_rate,
                skill.success_rate, false, false,
                skill.is_awakened, skill_json, now, now
            ],
        )?;
        Ok(())
    }

    /// Load all skills for a specialist
    pub fn load_skills(&self, specialist_id: &str) -> SqlResult<Vec<SkillRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, specialist_id, level, power_score, success_rate, 
                    breakthrough_eligible, breakthrough_achieved, awoken, skill_json
             FROM skills WHERE specialist_id = ?1 ORDER BY created_at DESC"
        )?;

        let skills = stmt.query_map(params![specialist_id], |row| {
            Ok(SkillRecord {
                id: row.get(0)?,
                specialist_id: row.get(1)?,
                level: row.get(2)?,
                power_score: row.get(3)?,
                success_rate: row.get(4)?,
                breakthrough_eligible: row.get(5)?,
                breakthrough_achieved: row.get(6)?,
                awoken: row.get(7)?,
                skill_json: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    /// Record an event in the event history
    pub fn record_event(
        &self,
        specialist_id: &str,
        event_id: &str,
        event_type: &str,
        xp_gained: u32,
        quality_score: f64,
        description: &str,
        timestamp: i64,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let data_json = json!({
            "event_type": event_type,
            "quality_score": quality_score
        }).to_string();

        self.db.execute(
            "INSERT INTO events 
             (id, specialist_id, event_type, xp_gained, quality_score, description, data_json, timestamp, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event_id, specialist_id, event_type, xp_gained as i32, quality_score,
                description, data_json, timestamp, now
            ],
        )?;
        Ok(())
    }

    /// Load event history for a specialist
    pub fn load_events(&self, specialist_id: &str, limit: usize) -> SqlResult<Vec<EventRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, specialist_id, event_type, xp_gained, quality_score, description, timestamp
             FROM events WHERE specialist_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
        )?;

        let events = stmt.query_map(params![specialist_id, limit as i32], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                specialist_id: row.get(1)?,
                event_type: row.get(2)?,
                xp_gained: row.get(3)?,
                quality_score: row.get(4)?,
                description: row.get(5)?,
                timestamp: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Save constellation node
    pub fn save_constellation_node(
        &self,
        node_id: &str,
        node_type: &str,
        label: &str,
        description: Option<&str>,
        x: f64,
        y: f64,
        z: f64,
        weight: f64,
        specialist_id: Option<&str>,
        timestamp: i64,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let node_json = json!({
            "type": node_type,
            "label": label,
            "position": [x, y, z],
            "weight": weight
        }).to_string();

        self.db.execute(
            "INSERT OR REPLACE INTO constellation_nodes
             (id, node_type, label, description, position_x, position_y, position_z, 
              weight, specialist_id, timestamp, node_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                node_id, node_type, label, description, x, y, z, weight,
                specialist_id, timestamp, node_json, now
            ],
        )?;
        Ok(())
    }

    /// Load constellation for a specialist
    pub fn load_constellation(&self, specialist_id: &str) -> SqlResult<Vec<ConstellationRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, node_type, label, position_x, position_y, position_z, weight, timestamp
             FROM constellation_nodes WHERE specialist_id = ?1 ORDER BY timestamp DESC"
        )?;

        let nodes = stmt.query_map(params![specialist_id], |row| {
            Ok(ConstellationRecord {
                id: row.get(0)?,
                node_type: row.get(1)?,
                label: row.get(2)?,
                x: row.get(3)?,
                y: row.get(4)?,
                z: row.get(5)?,
                weight: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(nodes)
    }

    /// Record data ingestion
    pub fn record_ingestion(
        &self,
        specialist_id: &str,
        ingestion_id: &str,
        file_path: &str,
        file_format: &str,
        file_size: u64,
        checksum: &str,
        domain: &str,
        xp_generated: u32,
        quality_score: f64,
        timestamp: i64,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT INTO ingestion_records
             (id, specialist_id, file_path, file_format, file_size, checksum, domain,
              xp_generated, quality_score, timestamp, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                ingestion_id, specialist_id, file_path, file_format, file_size as i64,
                checksum, domain, xp_generated as i32, quality_score, timestamp, now
            ],
        )?;
        Ok(())
    }

    /// Load ingestion records for a specialist
    pub fn load_ingestions(&self, specialist_id: &str) -> SqlResult<Vec<IngestionRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, specialist_id, file_path, file_format, file_size, checksum, domain,
                    xp_generated, quality_score, timestamp
             FROM ingestion_records WHERE specialist_id = ?1 ORDER BY timestamp DESC"
        )?;

        let records = stmt.query_map(params![specialist_id], |row| {
            Ok(IngestionRecord {
                id: row.get(0)?,
                specialist_id: row.get(1)?,
                file_path: row.get(2)?,
                file_format: row.get(3)?,
                file_size: row.get(4)?,
                checksum: row.get(5)?,
                domain: row.get(6)?,
                xp_generated: row.get(7)?,
                quality_score: row.get(8)?,
                timestamp: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    /// Delete a specialist and all related data
    pub fn delete_specialist(&self, specialist_id: &str) -> SqlResult<()> {
        self.db.execute(
            "DELETE FROM specialists WHERE id = ?1",
            params![specialist_id],
        )?;
        Ok(())
    }

    /// Get statistics for the entire hive
    pub fn get_hive_statistics(&self) -> SqlResult<HiveStatistics> {
        let total_specialists: i32 = self.db.query_row(
            "SELECT COUNT(*) FROM specialists WHERE active = 1",
            [],
            |row| row.get(0)
        )?;

        let total_xp: i64 = self.db.query_row(
            "SELECT COALESCE(SUM(xp_total), 0) FROM specialists WHERE active = 1",
            [],
            |row| row.get(0)
        )?;

        let total_skills: i32 = self.db.query_row(
            "SELECT COUNT(*) FROM skills",
            [],
            |row| row.get(0)
        )?;

        let total_events: i32 = self.db.query_row(
            "SELECT COUNT(*) FROM events",
            [],
            |row| row.get(0)
        )?;

        Ok(HiveStatistics {
            total_specialists: total_specialists as u32,
            total_xp: total_xp as u32,
            total_skills: total_skills as u32,
            total_events: total_events as u32,
        })
    }
}

// Data structures for persistence

#[derive(Debug, Clone)]
pub struct SpecialistRecord {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub birth_timestamp: i64,
    pub generation_level: u32,
    pub xp: u32,
    pub xp_total: u32,
    pub current_level: u32,
    pub rank: u32,
    pub genetics_json: String,
    pub soul_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SkillRecord {
    pub id: String,
    pub specialist_id: String,
    pub level: u32,
    pub power_score: f64,
    pub success_rate: f64,
    pub breakthrough_eligible: bool,
    pub breakthrough_achieved: bool,
    pub awoken: bool,
    pub skill_json: String,
}

#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: String,
    pub specialist_id: String,
    pub event_type: String,
    pub xp_gained: u32,
    pub quality_score: f64,
    pub description: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct ConstellationRecord {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub weight: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct IngestionRecord {
    pub id: String,
    pub specialist_id: String,
    pub file_path: String,
    pub file_format: String,
    pub file_size: i64,
    pub checksum: String,
    pub domain: String,
    pub xp_generated: u32,
    pub quality_score: f64,
    pub timestamp: i64,
}

/// Memory entry record for persistence
#[derive(Debug, Clone)]
pub struct MemoryEntryRecord {
    pub id: String,
    pub specialist_id: String,
    pub memory_type: String,
    pub title: String,
    pub description: String,
    pub context: String,
    pub confidence: String,
    pub relevance_score: f64,
    pub usage_count: i32,
    pub tags: String,
    pub related_memories: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct HiveStatistics {
    pub total_specialists: u32,
    pub total_xp: u32,
    pub total_skills: u32,
    pub total_events: u32,
}

impl PersistenceManager {
    /// Save memory entry to database
    pub fn save_memory_entry(
        &self,
        id: &str,
        specialist_id: &str,
        memory_type: &str,
        title: &str,
        description: &str,
        context: &str,
        confidence: &str,
        relevance_score: f64,
        usage_count: i32,
        tags: &str,
        related_memories: &str,
        source: &str,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO memory_entries 
             (id, specialist_id, memory_type, title, description, context, confidence, 
              relevance_score, usage_count, tags, related_memories, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id, specialist_id, memory_type, title, description, context, confidence,
                relevance_score, usage_count, tags, related_memories, source, now, now
            ],
        )?;
        Ok(())
    }

    /// Load memory entries for a specialist
    pub fn load_specialist_memories(&self, specialist_id: &str) -> SqlResult<Vec<MemoryEntryRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, specialist_id, memory_type, title, description, context, confidence,
                    relevance_score, usage_count, tags, related_memories, source, created_at, updated_at
             FROM memory_entries WHERE specialist_id = ?1 ORDER BY created_at DESC"
        )?;

        let records = stmt.query_map(params![specialist_id], |row| {
            Ok(MemoryEntryRecord {
                id: row.get(0)?,
                specialist_id: row.get(1)?,
                memory_type: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                context: row.get(5)?,
                confidence: row.get(6)?,
                relevance_score: row.get(7)?,
                usage_count: row.get(8)?,
                tags: row.get(9)?,
                related_memories: row.get(10)?,
                source: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

        Ok(records)
    }

    /// Save decision record to database
    pub fn save_decision_record(
        &self,
        id: &str,
        specialist_id: &str,
        decision: &str,
        reasoning: &str,
        alternatives: &str,
        outcome_success: Option<bool>,
        outcome_description: Option<&str>,
        confidence_before: &str,
        confidence_after: Option<&str>,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO decision_records 
             (id, specialist_id, decision, reasoning, alternatives_considered,
              outcome_success, outcome_description, confidence_before, confidence_after,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id, specialist_id, decision, reasoning, alternatives,
                outcome_success, outcome_description, confidence_before, confidence_after,
                now, now
            ],
        )?;
        Ok(())
    }

    /// Save strategy to database
    pub fn save_strategy(
        &self,
        id: &str,
        specialist_id: &str,
        name: &str,
        description: &str,
        steps_json: &str,
        effectiveness_score: f64,
        success_count: i32,
        failure_count: i32,
        applicable_to: &str,
        prerequisites: &str,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO strategies 
             (id, specialist_id, name, description, steps, effectiveness_score,
              success_count, failure_count, applicable_to, prerequisites, created_at, last_used)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id, specialist_id, name, description, steps_json, effectiveness_score,
                success_count, failure_count, applicable_to, prerequisites, now, now
            ],
        )?;
        Ok(())
    }

    /// Save goal to database
    pub fn save_goal(
        &self,
        id: &str,
        specialist_id: &str,
        objective: &str,
        reason: &str,
        status: &str,
        priority: i32,
        progress_percentage: i32,
        blockers: &str,
        milestones: &str,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.db.execute(
            "INSERT OR REPLACE INTO goals 
             (id, specialist_id, objective, reason, status, priority, progress_percentage,
              blockers, milestones, created_at, target_completion, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL)",
            params![
                id, specialist_id, objective, reason, status, priority, progress_percentage,
                blockers, milestones, now
            ],
        )?;
        Ok(())
    }

    /// Load goals for a specialist
    pub fn load_specialist_goals(&self, specialist_id: &str) -> SqlResult<Vec<String>> {
        let mut stmt = self.db.prepare(
            "SELECT objective FROM goals WHERE specialist_id = ?1 AND status != 'Completed'"
        )?;

        let objectives = stmt.query_map(params![specialist_id], |row| {
            row.get(0)
        })?
        .collect::<SqlResult<Vec<_>>>()?;

        Ok(objectives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_manager_creation() {
        // Create an in-memory database for testing
        let manager = PersistenceManager::new(":memory:")
            .expect("Failed to create persistence manager");
        
        let stats = manager.get_hive_statistics()
            .expect("Failed to get statistics");
        
        assert_eq!(stats.total_specialists, 0);
        assert_eq!(stats.total_skills, 0);
    }

    #[test]
    fn test_save_and_load_specialist() {
        let manager = PersistenceManager::new(":memory:")
            .expect("Failed to create persistence manager");

        // Create dummy genome and soul
        let genome = SpecialistGenome::new(
            "test_id".to_string(),
            "TestSpecialist".to_string(),
            "base_model".to_string()
        );
        
        // Create a minimal soul structure
        use chrono::Utc;
        use crate::self_digestion::{PersonalitySoul, RelationalSoul, NarrativeSoul, ExperienceSoul};
        
        let soul = SpecialistSoul {
            specialist_id: "test_id".to_string(),
            personality_soul: PersonalitySoul {
                archetype: "Scholar".to_string(),
                big_five_openness: 0.8,
                big_five_conscientiousness: 0.7,
                big_five_extraversion: 0.5,
                big_five_agreeableness: 0.6,
                big_five_neuroticism: 0.3,
                quirks: vec![],
                core_values: vec![],
                conversation_style: "thoughtful".to_string(),
                decision_making_style: "analytical".to_string(),
                emotional_tendencies: vec![],
                growth_areas: vec![],
            },
            relational_soul: RelationalSoul {
                natural_allies: vec![],
                natural_tensions: vec![],
                peer_relationships: std::collections::HashMap::new(),
                collaboration_patterns: vec![],
                conflict_resolution_style: "direct".to_string(),
            },
            narrative_soul: NarrativeSoul {
                origin_story: "test".to_string(),
                self_conception: "learning".to_string(),
                personal_goals: vec![],
                narrative_arc: "growth".to_string(),
                philosophical_beliefs: vec![],
                favorite_topics: vec![],
                fears_and_hopes: "hope".to_string(),
            },
            experience_soul: ExperienceSoul {
                shared_memories: vec![],
                lessons_learned: vec![],
                achievements: vec![],
                relationship_evolution: std::collections::HashMap::new(),
                evolution_timeline: vec![],
            },
            created_at: Utc::now(),
            version: 1,
        };

        // Save specialist
        manager.save_specialist(
            "test_id",
            "TestSpecialist",
            "Scholar",
            0,
            1,
            100,
            100,
            1,
            1,
            &genome,
            &soul,
        ).expect("Failed to save specialist");

        // Load specialist
        let record = manager.load_specialist("test_id")
            .expect("Failed to load specialist")
            .expect("Specialist not found");

        assert_eq!(record.id, "test_id");
        assert_eq!(record.name, "TestSpecialist");
        assert_eq!(record.xp, 100);
    }

    #[test]
    fn test_hive_statistics() {
        let manager = PersistenceManager::new(":memory:")
            .expect("Failed to create persistence manager");

        let genome = SpecialistGenome::new(
            "test_1".to_string(),
            "Test1".to_string(),
            "base_model".to_string()
        );
        
        use chrono::Utc;
        use crate::self_digestion::{PersonalitySoul, RelationalSoul, NarrativeSoul, ExperienceSoul};
        
        let soul = SpecialistSoul {
            specialist_id: "test_1".to_string(),
            personality_soul: PersonalitySoul {
                archetype: "Scholar".to_string(),
                big_five_openness: 0.8,
                big_five_conscientiousness: 0.7,
                big_five_extraversion: 0.5,
                big_five_agreeableness: 0.6,
                big_five_neuroticism: 0.3,
                quirks: vec![],
                core_values: vec![],
                conversation_style: "thoughtful".to_string(),
                decision_making_style: "analytical".to_string(),
                emotional_tendencies: vec![],
                growth_areas: vec![],
            },
            relational_soul: RelationalSoul {
                natural_allies: vec![],
                natural_tensions: vec![],
                peer_relationships: std::collections::HashMap::new(),
                collaboration_patterns: vec![],
                conflict_resolution_style: "direct".to_string(),
            },
            narrative_soul: NarrativeSoul {
                origin_story: "test".to_string(),
                self_conception: "learning".to_string(),
                personal_goals: vec![],
                narrative_arc: "growth".to_string(),
                philosophical_beliefs: vec![],
                favorite_topics: vec![],
                fears_and_hopes: "hope".to_string(),
            },
            experience_soul: ExperienceSoul {
                shared_memories: vec![],
                lessons_learned: vec![],
                achievements: vec![],
                relationship_evolution: std::collections::HashMap::new(),
                evolution_timeline: vec![],
            },
            created_at: Utc::now(),
            version: 1,
        };

        // Add a specialist
        manager.save_specialist(
            "test_1",
            "Test1",
            "Scholar",
            0,
            1,
            100,
            100,
            1,
            1,
            &genome,
            &soul,
        ).expect("Failed to save specialist");

        let stats = manager.get_hive_statistics()
            .expect("Failed to get statistics");

        assert_eq!(stats.total_specialists, 1);
        assert_eq!(stats.total_xp, 100);
    }
}
