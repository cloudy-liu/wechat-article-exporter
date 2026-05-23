# Use Tauri and Rust for the Desktop Backend

The desktop application will use Tauri 2 with the existing Vue/Nuxt interface compiled as a static frontend, while migrating Nitro server responsibilities into a Rust backend. This avoids shipping a Node/Nitro sidecar and keeps the author-facing app closer to a native desktop product, at the cost of rewriting login, cookie storage, WeChat request proxying, download tasks, and local persistence behind Tauri commands.

**Considered Options**

- Bundle the existing Nuxt/Nitro service as a local Node sidecar to maximize short-term reuse.
- Migrate server behavior into Rust and call it from the frontend through Tauri IPC.

**Consequences**

- The existing UI can be reused incrementally, but `/api/web/...` calls must move behind desktop service APIs.
- Local persistence should move toward SQLite and filesystem-backed assets instead of relying on server KV or browser-only IndexedDB as the primary desktop store.
