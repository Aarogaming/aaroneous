# Security Policy

## Supported Versions

| Version | Supported          |
|---------|-------------------|
| 0.4.x   | :white_check_mark: |
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability within Aaroneous, please send an email to the maintainers. All security vulnerabilities will be promptly addressed.

**Please do NOT report security vulnerabilities through public GitHub issues.**

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix or mitigation**: Within 2 weeks for critical vulnerabilities

## Security Architecture & Containment Hardening

Aaroneous operates with elevated system capabilities (Win32 HID interception, desktop automation, memory-mapped I/O). Key security boundaries include:

- **Workspace Path Containment Jail**: Canonical path verification in `ActionExecutor::validate_sandbox_path` enforcing strict boundary isolation against directory traversal (`../`).
- **Sandboxed Pure-Rust Micro-VM**: Gas-metered (`VmError::GasExhausted`) and bounds-checked linear memory execution (`VmError::MemoryOutOfBounds`) for untrusted dynamic plugins.
- **Constant-Time Authentication**: Side-channel resistant token comparison via `subtle::ConstantTimeEq` on all protected endpoints.
- **Named Pipe Client Restriction**: Localhost impersonation enforcement rejecting remote network clients in `ipc_bus::comm`.
- **HID Emergency Cursor Failsafe**: Instant mouse/keyboard input release upon cursor traversal to the screen boundary corner.
- **Isolated Desktop Sandbox**: Win32 `CreateDesktopW` sandboxing for kinetic execution.
- **Sentinel Anomaly Detection**: Deep SVDD anomaly detection and safe hypersphere manifold snapping on latent agent vectors.
- **SHA-256 System Integrity Monitoring**: Automatic hash verification of workspace files and in-memory credential hashing.

## Best Practices

- Run with minimum required privileges
- Use Isolated Desktop isolation for untrusted workloads
- Enable Sentinel security monitoring in production
- Keep `.si` model files verified with SHA256 hashes
