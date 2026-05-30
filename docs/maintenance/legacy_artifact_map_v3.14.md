# Legacy Artifact Map v3.14

This document outlines the legacy Python artifacts discovered during the audit and their upgrade paths to the 3.14.5 release.

## Python Artifacts Found

### AAS_Core
- **Location**: `D:\Aaroneous\legacy\aas_core`
- **Status**: Deprecated, requires modernization
- **Upgrade Path**: 
  - Migrate to new free-threaded concurrency mode
  - Replace GIL-dependent patterns with async/await
  - Update to use modern Python 3.14 features

### Wizard101_DanceBot
- **Location**: `D:\Aaroneous\legacy\wizard101_dancebot`
- **Status**: Legacy automation scripts
- **Upgrade Path**:
  - Refactor macro loops to use 3.14's GIL-free parallelism
  - Update to use new concurrency primitives
  - Remove outdated dependency on Python 2.7

### android-app
- **Location**: `D:\Aaroneous\legacy\android-app`
- **Status**: Obsolete Android application
- **Upgrade Path**:
  - Migrate to modern Android development practices
  - Update to use current Android SDKs
  - Implement new UI/UX patterns

## Migration Strategy

1. **Odin Shard**:
   - Modernize automation/macro loops to leverage 3.14's GIL-free parallelism
   - Replace deprecated threading patterns with asyncio

2. **Ariel Shard**:
   - Update UI/UX manifest logic to use new t-string literals
   - Migrate legacy UI components to new design system

3. **Dionysus Shard**:
   - Implement ingestion/record-keeping patterns with native zstd support
   - Update data handling to leverage new compression features

## Next Steps

1. Begin refactoring of AAS_Core for free-threaded concurrency
2. Update Wizard101_DanceBot to use new async patterns
3. Modernize android-app with current Android development practices