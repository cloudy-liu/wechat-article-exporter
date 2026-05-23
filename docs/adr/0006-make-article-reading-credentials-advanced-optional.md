# Make Article Reading Credentials Advanced and Optional

The first desktop release must not require article reading credentials for the core collection workflow. Official Account platform login is required for target account search and article-list synchronization, while article reading credentials are optional enrichment for read counts, likes, comments, and some paid article content.

**Consequences**

- Markdown and HTML collection must work without configuring wxdown-service, mitmproxy, or any other credential-capture helper.
- UI copy should distinguish **Official Account Login** from **Article Reading Credential** instead of using the generic word "Credential".
- Advanced enrichment can be preserved and improved later without blocking the main desktop experience.
