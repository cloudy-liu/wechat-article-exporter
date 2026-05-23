# Migrate Only the Desktop Core Feature Set First

The first desktop release will migrate target account management, account article download, single article download, album download, and settings. API documentation, public proxy operations, sponsorship/support pages, and developer/debug pages are web-service or project-operation concerns and will not be migrated into the first desktop product.

**Consequences**

- The desktop navigation should be smaller than the current web navigation.
- The first-release scope follows the local collection and archival workflow rather than full web parity.
- Removed web-only pages can stay in the repository for the web app while being excluded from the desktop shell.
