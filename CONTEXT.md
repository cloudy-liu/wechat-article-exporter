# WeChat Article Exporter

This context describes a local desktop tool for WeChat Official Account operators to collect, cache, study, and archive public WeChat Official Account content.

## Language

**Desktop Archive App**:
A local desktop application used by a WeChat Official Account operator to collect and archive public official account content.
_Avoid_: public API platform, web deployment, all-in-one platform

**Legacy Web App**:
The existing Nuxt/Nitro web implementation kept only as a reference for behavior and migration.
_Avoid_: active product, deployment target

**Desktop Workspace**:
The active application workspace containing the Tauri, Rust, Vite, and Vue desktop product.
_Avoid_: root Nuxt rewrite, legacy web app

**Desktop Frontend**:
The Vite, Vue 3, and vue-router application shell used by the active desktop product.
_Avoid_: Nuxt app shell, Nitro-coupled frontend

**Platform Architecture**:
The desktop architecture must avoid Windows-only assumptions so the app can later ship on macOS and Linux.
_Avoid_: Windows-only design, hard-coded local paths

**First Release Platform**:
The operating system target used to accept the first desktop release: Windows x64.
_Avoid_: all-platform release blocker

**WeChat Official Account Operator**:
The person operating or studying WeChat Official Accounts who logs in locally and exports public account content for research, reference, and archival workflows.
_Avoid_: developer user, API client, third-party integrator

**Target Official Account**:
A WeChat Official Account selected for collection, whether it is operated by the local user or by someone else.
_Avoid_: own account, source account

**Official Account Login**:
A local login session established by scanning with an account that can access the WeChat Official Account platform.
_Avoid_: personal WeChat login, ordinary WeChat user login

**Article Reading Credential**:
A short-lived reading-side credential used for advanced data such as read counts, likes, comments, and some paid article content.
_Avoid_: official account login, generic credential

**Article Archive**:
A local copy of public official account article content and selected metadata intended for long-term retrieval and downstream processing.
_Avoid_: scraped dataset, public mirror

**Core Export Format**:
The export formats that define the first desktop release: Markdown and HTML.
_Avoid_: PDF as a first-release requirement

**Auxiliary Export Format**:
Useful non-core export formats that should be preserved when practical: Excel, JSON, and TXT.
_Avoid_: release blocker, primary archive format

**Deferred Export Format**:
Export formats that are not first-release requirements: Word and PDF.
_Avoid_: core export format, required parity

**Public API**:
A programmatic interface for third-party callers to search accounts, fetch article lists, or download content without using the desktop UI.
_Avoid_: desktop workflow, author workflow

**Proxy Monitoring**:
Operational visibility for public proxy infrastructure, not part of the author-facing desktop archival workflow.
_Avoid_: desktop status, download progress

**Local Download Engine**:
The desktop backend component that downloads article HTML, article assets, metadata, and comments directly from the local machine.
_Avoid_: public proxy pool, browser download proxy

**Local Archive Store**:
The desktop application's durable local storage made of SQLite for structured records and filesystem directories for article files and assets.
_Avoid_: browser cache, IndexedDB as primary storage

**Secret Store**:
The encrypted local storage used for official account login cookies, tokens, and article reading credentials.
_Avoid_: plaintext SQLite, plaintext config file, browser localStorage

**Persistent Collection Task**:
A durable local task that tracks account sync, article download, album download, or export progress across application restarts.
_Avoid_: in-memory queue, byte-level resumable download

**Network Proxy Setting**:
An advanced desktop network configuration for routing local download traffic through a user-provided system, HTTP, or SOCKS proxy.
_Avoid_: public proxy, proxy monitoring

**Original Feature Baseline**:
The current web project's user-facing collection, caching, and export capabilities, excluding web-service-only or deployment-operation features.
_Avoid_: complete web parity, public service parity

**Desktop Core Feature Set**:
The first-release desktop features: target account management, account article download, single article download, album download, and settings.
_Avoid_: public API docs, public proxy dashboard, sponsorship page, developer pages

## Relationships

- A **WeChat Official Account Operator** uses one **Desktop Archive App** on their local machine.
- The **Desktop Archive App** is the only active product.
- The **Desktop Archive App** lives in the **Desktop Workspace**.
- The **Desktop Archive App** uses the **Desktop Frontend** for its user interface.
- The **Desktop Archive App** follows the **Platform Architecture** but is accepted first on the **First Release Platform**.
- The **Legacy Web App** may inform migration but must not drive new architecture decisions.
- A **Desktop Archive App** requires an **Official Account Login** before searching target accounts or syncing article lists.
- An **Article Reading Credential** may enrich an **Article Archive**, but the core Markdown/HTML collection workflow must not depend on it.
- A **WeChat Official Account Operator** selects one or more **Target Official Accounts**.
- A **Desktop Archive App** produces one or more **Article Archives** from a **Target Official Account**.
- A **Desktop Archive App** uses the **Local Download Engine** by default.
- A **Desktop Archive App** stores durable collection data in the **Local Archive Store**.
- A **Desktop Archive App** stores sensitive login and reading data in the **Secret Store**.
- A **Persistent Collection Task** records progress in the **Local Archive Store**.
- A **Network Proxy Setting** may change the network route used by the **Local Download Engine**.
- An **Article Archive** is exported through one **Core Export Format** at a time.
- An **Auxiliary Export Format** may support analysis workflows, but it does not define first-release success.
- A **Deferred Export Format** must not delay first-release desktop delivery.
- The **Original Feature Baseline** guides desktop migration scope.
- The **Desktop Core Feature Set** is the first-release migration target.
- **Public API** and **Proxy Monitoring** are outside the first desktop release unless explicitly reintroduced later.

## Example Dialogue

> **Dev:** "Is the desktop app mainly for backing up the user's own account?"
> **Domain expert:** "No. It must support collecting public articles from other target official accounts; self-archival is included, but not sufficient."

> **Dev:** "Can ordinary personal WeChat users scan in?"
> **Domain expert:** "No. The desktop app is for users with access to the WeChat Official Account platform."

> **Dev:** "Do read counts and comments have to work before a user can export Markdown?"
> **Domain expert:** "No. Those require an article reading credential and are advanced optional enrichment."

> **Dev:** "Does PDF parity block the desktop release?"
> **Domain expert:** "No. Markdown and HTML are the core formats; Excel, JSON, and TXT are useful, while Word and PDF can wait."

> **Dev:** "Which existing pages should move into the desktop app?"
> **Domain expert:** "Move the account, article, single-article, album, and settings workflows; drop API docs, public proxy operations, sponsorship, and dev pages."

> **Dev:** "Do new features need to keep the web deployment working?"
> **Domain expert:** "No. The desktop app is the only product; the web version is only a reference during migration."

> **Dev:** "Should we rewrite the root Nuxt app in place?"
> **Domain expert:** "No. Create a desktop workspace and keep the old web app as a frozen migration reference."

> **Dev:** "Should the active desktop UI keep using Nuxt?"
> **Domain expert:** "No. Use a Vite + Vue 3 desktop frontend and migrate behavior from the Nuxt web app as needed."

> **Dev:** "Can we hard-code Windows paths because first release is Windows?"
> **Domain expert:** "No. The first release is accepted on Windows, but the architecture should remain multi-platform."

> **Dev:** "Should we keep the public proxy page in the desktop app?"
> **Domain expert:** "No. The desktop app should download locally by default and only expose proxy configuration as an advanced network setting."

> **Dev:** "Should the desktop app keep IndexedDB as the main business database?"
> **Domain expert:** "No. Use SQLite for structured data and local files for article content and assets."

> **Dev:** "Can we store WeChat cookies directly in SQLite?"
> **Domain expert:** "No. Login cookies, tokens, and article reading credentials belong in the encrypted secret store."

> **Dev:** "Do downloads need byte-level resume?"
> **Domain expert:** "No. Persist task and item state so failed articles or resources can be retried after restart."

## Flagged Ambiguities

- "保留所有功能" was narrowed to mean preserving the author-facing collection, caching, and export workflow; public API, public proxy monitoring, and PDF export are not first-release requirements.
- "公众号作者本地用" does not mean "own-account backup only"; the resolved product scope is public official account collection, where collecting other accounts is a core workflow.
- "扫码登录" means **Official Account Login**, not ordinary personal WeChat login.
- "Credential" was split into **Official Account Login** and **Article Reading Credential**; only the former is required for the first-release collection workflow.
- "代理" in the desktop product means **Network Proxy Setting**, not public proxy pool operation or monitoring.
- "本地数据" means the **Local Archive Store**, not browser-only IndexedDB/Dexie as the primary desktop data store.
- "保存登录态" means encrypted persistence in the **Secret Store**, not plaintext storage.
- "断点续传" means **Persistent Collection Task** recovery at article/resource/export-item level, not HTTP byte-range resume.
- "所有导出功能保留" was resolved into **Core Export Format**, **Auxiliary Export Format**, and **Deferred Export Format** rather than equal-priority parity.
- "原版内容保留" means preserving the **Desktop Core Feature Set**, not migrating web-service-only pages.
- "Web 版本保留" means **Legacy Web App** reference only, not an active product line.
- "代码库重构" means creating the **Desktop Workspace** for the active product, not mutating the legacy Nuxt app in place.
- "前端保留" means preserving useful Vue behavior and UI patterns, not keeping Nuxt as the active app shell.
- "第一版 Windows" means the **First Release Platform** for acceptance, not a Windows-only **Platform Architecture**.
