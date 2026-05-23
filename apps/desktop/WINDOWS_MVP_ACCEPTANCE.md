# Windows MVP Acceptance

This document is the handoff checklist for issue #21. It records how to build the Windows x64 desktop artifact, how to verify it locally before owner acceptance, and which first-release limitations remain known.

## Build Artifact

From the repository root:

```powershell
yarn --cwd apps/desktop build:windows-mvp
yarn --cwd apps/desktop verify:windows-mvp --launch-smoke
```

`build:windows-mvp` runs `tauri build --no-bundle` so the required runnable artifact can be produced without depending on installer tooling.

The required Windows x64 desktop artifact is a runnable Windows artifact:

- `apps/desktop/src-tauri/target/release/wechat-article-exporter-desktop.exe`

An installer bundle is optional for the MVP handoff. If local WiX or NSIS bundling is available, `yarn --cwd apps/desktop tauri:build` may also produce an installable bundle under `apps/desktop/src-tauri/target/release/bundle`, such as an `.msi` or installer `.exe`. The automated verifier supports this stricter check with `--require-installer`.

The packaged app launches on Windows when `verify:windows-mvp --launch-smoke` can start the release executable and keep it alive long enough for the smoke check.

## Core Workflow Verification

Use the packaged Windows app for the final local acceptance pass. The verification flow must cover:

- Official Account Login: scan with a WeChat Official Account platform operator account and reach a logged-in state.
- Target Official Account search: search for a target account, add it to the local target list, and confirm it remains listed.
- article list synchronization: sync target account article history and confirm the local article list is populated.
- article HTML download: download at least one synced article and confirm an archived HTML file is recorded.
- Markdown export: export a downloaded article to Markdown and confirm the local export path exists.
- HTML export: export a downloaded article to HTML and confirm the local export path exists.

## Restart And Credential Checks

Restart behavior must preserve local archive data and task visibility:

- Run at least one sync, article HTML download, and export task.
- Close the packaged app.
- Reopen the packaged app.
- Confirm target accounts, article records, local archive paths, and collection task visibility are still present.

Logout and credential clearing must work in the packaged app:

- Use the settings page to run Logout from the Official Account session.
- Use the settings page to run credential clearing.
- Confirm core local archive data remains visible after credential clearing, while credential-dependent operations require login again.

## Known First-Release Limitations

- Windows x64 is the first release acceptance target; macOS and Linux packaging remain later verification targets.
- The runnable release `.exe` is the required MVP artifact; installer bundle is optional because the default MSI bundle path may require downloading external WiX tooling on a clean Windows machine.
- The app requires a WeChat Official Account platform operator login for target account search and article list synchronization.
- Article Reading Credentials are advanced optional enrichment and are not required for Markdown or HTML collection.
- PDF and Word exports are deferred; Markdown and HTML are the core first-release formats.
- Public API pages, public proxy pool operations, sponsorship/support pages, developer/debug pages, hosted analytics, and Cloudflare worker dashboards are outside the desktop MVP.

## Owner Acceptance

After the automated/local verification path passes, the produced Windows x64 desktop artifact is ready for final local acceptance by the project owner.
