# Create a Desktop Workspace and Freeze the Legacy Web App

The active product will be built in a new desktop workspace rather than rewriting the root Nuxt/Nitro app in place. The existing web application should be preserved as a frozen legacy reference for behavior migration while the desktop workspace grows around Tauri, Rust, Vite, Vue, SQLite, local files, secret storage, and persistent tasks.

**Consequences**

- The legacy web implementation can be consulted without continuing to drive product architecture.
- Desktop code can start from the correct assumptions instead of carrying Nuxt, Nitro, public APIs, public proxy operations, browser storage, and web deployment concerns forward.
- Migration can proceed by vertical slices from the legacy behavior into the desktop workspace.
