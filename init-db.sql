-- Aaroneous Federation Database Initialization
-- This script sets up the initial schema and tables

CREATE SCHEMA IF NOT EXISTS aaroneous;

-- DNA Bank table for storing genetic information
CREATE TABLE IF NOT EXISTS aaroneous.dna_bank (
    id SERIAL PRIMARY KEY,
    chromosome_id UUID NOT NULL,
    gene_sequence TEXT NOT NULL,
    traits JSONB,
    fitness_score FLOAT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Audit log table
CREATE TABLE IF NOT EXISTS aaroneous.audit_logs (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(255) NOT NULL,
    event_data JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Specialist registry
CREATE TABLE IF NOT EXISTS aaroneous.specialist_registry (
    id SERIAL PRIMARY KEY,
    specialist_type VARCHAR(100) NOT NULL UNIQUE,
    config JSONB,
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Federation peers
CREATE TABLE IF NOT EXISTS aaroneous.federation_peers (
    id SERIAL PRIMARY KEY,
    peer_id UUID NOT NULL UNIQUE,
    peer_address VARCHAR(255) NOT NULL,
    consensus_votes INTEGER DEFAULT 0,
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(50) DEFAULT 'active'
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_dna_bank_chromosome ON aaroneous.dna_bank(chromosome_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON aaroneous.audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_federation_peers_peer_id ON aaroneous.federation_peers(peer_id);

-- Insert default specialist configurations
INSERT INTO aaroneous.specialist_registry (specialist_type, config, enabled) VALUES
    ('sentinel', '{"instances": 3, "enabled": true}'::jsonb, true),
    ('visionary', '{"enabled": true}'::jsonb, true),
    ('omnipresent', '{"enabled": true}'::jsonb, true),
    ('symbiotic', '{"enabled": true}'::jsonb, true),
    ('phygital', '{"enabled": true}'::jsonb, true),
    ('archivist', '{"enabled": true}'::jsonb, true)
ON CONFLICT (specialist_type) DO NOTHING;
