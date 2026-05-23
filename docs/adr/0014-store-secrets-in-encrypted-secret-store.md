# Store Secrets in an Encrypted Secret Store

The desktop application may persist Official Account login cookies, tokens, and Article Reading Credentials so users do not need to reconfigure them on every launch. These sensitive values must be stored in an encrypted secret store rather than plaintext SQLite, local files, browser localStorage, or ordinary application settings.

**Consequences**

- SQLite can store non-sensitive records and references, but not plaintext WeChat cookies or tokens.
- The app must provide logout and clear-local-credentials actions.
- The first Windows release can use a Windows-backed secret storage approach while keeping the abstraction portable for macOS Keychain, Linux Secret Service, or equivalent secure storage later.
