# Make Desktop the Only Active Product

The project will treat the Tauri desktop application as the only active product. The existing Nuxt/Nitro web application may remain in the repository as a behavior reference and migration source, but new architecture decisions do not need to preserve web deployment, public web APIs, Nitro hosting, browser storage, or public proxy operations.

**Consequences**

- Desktop reliability and local usability take precedence over web compatibility.
- Existing web code can be copied, adapted, or retired as migration progresses.
- Documentation, navigation, and build scripts should eventually reflect the desktop-first product shape.
