# Security Policy

## Supported Versions

| Version | Supported          |
|---------|-------------------|
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

## Security Considerations

Aaroneous operates with elevated system privileges (Win32 HID interception, desktop automation, memory-mapped I/O). Key security boundaries:

- **Isolated Desktop Isolation**: Win32 `CreateDesktopW` sandboxing for kinetic execution
- **Sentinel**: Deep SVDD anomaly detection on all agent outputs
- **Capability restrictions**: Agents have explicit `enzyme_subset` allowlists
- **No network exposure by default**: Federation requires explicit `--bind` flag

## Best Practices

- Run with minimum required privileges
- Use Isolated Desktop isolation for untrusted workloads
- Enable Sentinel security monitoring in production
- Keep `.si` model files verified with SHA256 hashes
