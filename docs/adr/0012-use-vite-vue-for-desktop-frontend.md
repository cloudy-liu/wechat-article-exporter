# Use Vite and Vue for the Desktop Frontend

The active desktop product will use a Vite + Vue 3 + vue-router frontend instead of keeping Nuxt as the application shell. The existing Nuxt/Nitro app remains a migration reference for behavior and UI patterns, but Nitro routes, web deployment configuration, and Nuxt-specific runtime assumptions should not shape the desktop architecture.

**Consequences**

- Desktop pages should be rebuilt around the core routes: target accounts, articles, single article, albums, and settings.
- Existing Vue components and Composition API logic can be adapted, but calls to `/api/web/...` must move behind Tauri commands.
- Nuxt modules tied to web analytics, server routes, or hosted deployment can be removed from the active desktop app.
