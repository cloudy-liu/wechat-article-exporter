# Design Cross-Platform but Accept First on Windows

The desktop architecture must avoid Windows-only assumptions so macOS and Linux builds remain possible later. The first release is accepted on Windows x64, but storage paths, file APIs, network proxy configuration, and packaging boundaries should use Tauri and Rust abstractions rather than hard-coded Windows behavior.

**Consequences**

- Windows is the first verification and packaging target.
- Cross-platform portability remains an architectural constraint.
- macOS and Linux release work can be deferred without forcing a later rewrite.
