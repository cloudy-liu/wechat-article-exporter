# Use a Local Download Engine Instead of the Public Proxy Pool

The desktop application will download article HTML, resources, metadata, and comments through the local Rust backend by default instead of depending on the web project's public proxy pool. Public proxy monitoring and worker operational pages are web-service concerns, while the desktop product only needs an advanced network proxy setting for users who must route traffic through a system, HTTP, or SOCKS proxy.

**Consequences**

- The public proxy dashboard and worker metrics are not migrated to the desktop first release.
- The Rust backend replaces `/api/web/proxy/download` by issuing outbound requests directly with the required headers, cookies, retry policy, and timeout handling.
- Proxy configuration remains available as an advanced network setting, not as a primary workflow.
