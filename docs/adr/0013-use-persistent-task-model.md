# Use a Persistent Task Model

The desktop application will persist account sync, article download, album download, and export task progress in the local archive store. Recovery is at the task-item level, such as retrying failed articles or resources after restart, not byte-level HTTP range resume.

**Consequences**

- Users can close or restart the app without losing visibility into long-running collection work.
- Failed items can be retried without repeating all completed work.
- The Rust backend owns task scheduling, status updates, cancellation, and retry behavior.
