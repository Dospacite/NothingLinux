# Security Policy

## Supported versions

Only the current `main` branch is supported before the project reaches a stable release series.

## Reporting vulnerabilities

Please report security issues privately through GitHub Security Advisories if available, or by opening a minimal issue that does not include exploit details.

Useful reports include:

- affected commit or release;
- Linux distribution and BlueZ version;
- whether the issue requires a paired device;
- sanitized logs with Bluetooth addresses, serial numbers, and local paths redacted.

## Scope

Security-sensitive areas include Bluetooth discovery/session handling, command validation, persistence, autostart, diagnostics redaction, and any code that could send mutating device commands.
