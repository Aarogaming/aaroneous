#!/usr/bin/env node
/**
 * Aaroneous Configuration Validation & Versioning System
 *
 * Validates all configuration files for:
 * - Schema compliance
 * - Semantic versioning consistency
 * - Required field presence
 * - Type validation
 * - Cross-reference integrity
 */

const Ajv = require('ajv');
const fs = require('fs');
const path = require('path');

// Initialize AJV validator with strict options
const ajv = new Ajv({ allErrors: true, strict: true });

// Schema registry for configuration validation
const schemas = {};

/**
 * Define JSON Schemas for configuration validation
 */
function loadSchemas() {
    // Runtime Manifest Schema
    const runtimeManifestSchema = {
        type: 'object',
        required: ['schema_version', 'runtime_version', 'bootstrap'],
        properties: {
            schema_version: { type: 'string', pattern: '^\\d+\\.\\d+\\.\\d+$' },
            runtime_version: { type: 'string' },
            product: { type: 'string' },
            designation: { type: 'string' },
            bootstrap: {
                type: 'object',
                required: ['entry_point', 'service_name'],
                properties: {
                    entry_point: { type: 'string' },
                    service_name: { type: 'string' },
                    startup_type: { type: 'string', enum: ['auto', 'manual'] },
                    run_as: { type: 'string' }
                }
            },
            directories: { type: 'object' },
            dependencies: { type: 'object' },
            security_ops: { type: 'object' }
        }
    };

    // Specialist Registry Schema
    const specialistRegistrySchema = {
        type: 'object',
        required: ['schema_version', 'federation_core_specialists'],
        properties: {
            schema_version: { type: 'string', pattern: '^\\d+\\.\\d+\\.\\d+$' },
            timestamp: { type: 'string', format: 'date-time' },
            federation_core_specialists: { type: 'object' }
        }
    };

    // Spectrum Config Schema
    const spectrumConfigSchema = {
        type: 'object',
        required: ['schema_version', 'specialist_profiles'],
        properties: {
            schema_version: { type: 'string', pattern: '^\\d+\\.\\d+\\.\\d+$' },
            timestamp: { type: 'string' },
            specialist_profiles: {
                type: 'object',
                additionalProperties: {
                    type: 'object',
                    required: ['role', 'domain']
                }
            }
        }
    };

    // District Architecture Schema
    const districtArchSchema = {
        type: 'object',
        required: ['schema_version', 'districts'],
        properties: {
            schema_version: { type: 'string' },
            districts: {
                type: 'object',
                additionalProperties: {
                    type: 'object',
                    required: ['name', 'primary_agent']
                }
            }
        }
    };

    // Omni Node Types Schema
    const omniNodeSchema = {
        type: 'object',
        required: ['schema_version', 'node_categories'],
        properties: {
            schema_version: { type: 'string' },
            node_categories: { type: 'object' }
        }
    };

    // Register schemas with AJV
    schemas.runtime_manifest = ajv.compile(runtimeManifestSchema);
    schemas.specialist_registry = ajv.compile(specialistRegistrySchema);
    schemas.spectrum_config = ajv.compile(spectrumConfigSchema);
    schemas.district_architecture = ajv.compile(districtArchSchema);
    schemas.omni_node_types = ajv.compile(omniNodeSchema);

    console.log('✅ Loaded 5 configuration schemas');
}

/**
 * Validate a single configuration file
 */
function validateConfig(filePath, schemaName) {
    try {
        const content = fs.readFileSync(filePath, 'utf8');
        const config = JSON.parse(content);

        // Check for required fields before full validation
        if (!config.schema_version) {
            console.error(`❌ ${path.basename(filePath)}: Missing schema_version field`);
            return false;
        }

        // Validate against schema
        const validator = schemas[schemaName];
        if (!validator) {
            console.warn(`⚠️  No validator found for ${schemaName}`);
            return true; // Skip validation for unknown types
        }

        const valid = validator(config);

        if (valid) {
            console.log(`✅ ${path.basename(filePath)}: Valid (${config.schema_version})`);

            // Check version consistency
            checkVersionConsistency(filePath, config);

            return true;
        } else {
            console.error(`❌ ${path.basename(filePath)}: Schema validation failed`);
            validator.errors.forEach(error => {
                console.error(`   - ${error.instancePath || ''}: ${error.message}`);
            });
            return false;
        }
    } catch (err) {
        if (err.code === 'ENOENT') {
            console.warn(`⚠️  ${path.basename(filePath)}: File not found`);
        } else if (err.name === 'SyntaxError') {
            console.error(`❌ ${path.basename(filePath)}: Invalid JSON - ${err.message}`);
        } else {
            console.error(`❌ ${path.basename(filePath)}: Unexpected error - ${err.message}`);
        }
        return false;
    }
}

/**
 * Check version consistency across related files
 */
function checkVersionConsistency(filePath, config) {
    const baseName = path.basename(filePath, '.json');

    // Check if timestamp is reasonable (not in future)
    if (config.timestamp) {
        const now = new Date();
        const fileTime = new Date(config.timestamp);

        if (fileTime > now) {
            console.warn(`   ⚠️  Timestamp ${config.timestamp} is in the future`);
        } else {
            const diffDays = Math.floor((now - fileTime) / (1000 * 60 * 60 * 24));
            if (diffDays > 30) {
                console.warn(`   ⚠️  Configuration not updated in ${diffDays} days`);
            }
        }
    }

    // Check for deprecated fields (example check - expand as needed)
    if (config.specialist_profiles) {
        Object.entries(config.specialist_profiles).forEach(([name, profile]) => {
            if (!profile.startup_order) {
                console.warn(`   ⚠️  Specialist '${name}' missing startup_order`);
            }
        });
    }
}

/**
 * Scan directory for configuration files and validate them
 */
function scanAndValidate(configDir, schemaMappings) {
    const results = {
        valid: [],
        invalid: [],
        warnings: []
    };

    console.log(`\n🔍 Scanning ${configDir}...\n`);

    fs.readdirSync(configDir).forEach(filename => {
        if (!filename.endsWith('.json')) return;

        const filePath = path.join(configDir, filename);

        // Map file to schema based on naming convention
        let schemaName = null;
        for (const [key, pattern] of Object.entries(schemaMappings)) {
            if (filename.includes(pattern)) {
                schemaName = key;
                break;
