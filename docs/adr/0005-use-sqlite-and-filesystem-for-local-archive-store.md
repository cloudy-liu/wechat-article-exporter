# Use SQLite and Filesystem for the Local Archive Store

The desktop application will use SQLite for structured collection data and filesystem directories for article HTML, Markdown, exported files, downloaded assets, and debug samples. IndexedDB/Dexie may remain as a temporary migration aid or for non-critical UI state, but it will not be the primary store for desktop business data.

**Consequences**

- The Rust backend becomes the primary writer for target accounts, article lists, content caches, metadata, comments, and task state.
- Large blobs should be stored as files referenced by SQLite records rather than embedded in browser storage.
- Backup, restore, and cross-machine migration can be designed around a local archive directory instead of browser-profile data.
