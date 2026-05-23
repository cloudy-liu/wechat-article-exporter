# Desktop Workspace

This directory is the active product workspace for the WeChat Article Exporter Desktop Archive App.

The desktop product will be built with Tauri 2, Rust, Vite, Vue 3, and vue-router. New implementation work should start here rather than in the legacy Nuxt/Nitro web application.

## Current Status

This workspace is intentionally minimal for issue #5. The runnable Tauri shell is tracked separately in issue #6.

## Product Scope

- Target account management
- Account article download
- Single article download
- Album download
- Desktop settings

Out of scope for this workspace: public API pages, public proxy operations, hosted web deployment, sponsorship/support pages, and developer/debug pages.
