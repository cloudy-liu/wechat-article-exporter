# Avoid AG Grid Enterprise in the Desktop App

The desktop application will not depend on AG Grid Enterprise unless a valid commercial license explicitly covers desktop distribution. The first desktop release should use AG Grid Community or an equivalent free table capability, replacing Enterprise-only conveniences such as the columns tool panel with lightweight in-app controls if needed.

**Consequences**

- Desktop distribution is not blocked by an enterprise grid license key.
- Existing grid-heavy workflows can be migrated incrementally by keeping the community grid API where possible.
- Any Enterprise-only behavior must be removed, replaced, or treated as non-core.
