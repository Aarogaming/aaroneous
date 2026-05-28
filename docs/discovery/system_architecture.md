# System Architecture: Hox Registry and Retina Modules

## Hox Registry

The Hox Registry is a capability management system that stores and retrieves information about registered capabilities in the system. It uses SQLite as its backend for persistent storage.

### Key Features
- Stores Hox capabilities with their enzyme hashes and permissions
- Provides methods for registering, retrieving, and listing capabilities
- Supports upsert operations for capability updates
- Implements proper error handling for database operations

### Implementation Details
- Uses `rusqlite` for SQLite operations
- Implements `parking_lot::Mutex` for thread-safe database access
- Stores permissions as structured data in JSON format
- Handles database schema initialization on startup

### Data Model
```sql
CREATE TABLE hox_capabilities (
    name TEXT PRIMARY KEY,
    enzyme_hash TEXT NOT NULL,
    permissions_json TEXT NOT NULL
);
```

## Retina Module

The Retina module serves as the zero-copy web ingestion system that processes web content for the system.

### Key Features
- Web content ingestion from URLs
- HTML rendering and content extraction
- Text extraction with boilerplate removal
- Zero-copy tokenization into Synapse format
- Compliance checking for web content

### Implementation Details
- Uses Playwright for browser automation
- Implements tokenization with tokenizers crate
- Extracts text content using regex patterns
- Integrates with the synapse system for zero-copy data transfer

### Data Flow
1. URL ingestion
2. Browser rendering with human-like wait times
3. HTML content extraction
4. Boilerplate stripping
5. Tokenization into Synapse format
6. Synapse buffer population