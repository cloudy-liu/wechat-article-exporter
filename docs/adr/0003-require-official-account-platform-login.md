# Require WeChat Official Account Platform Login

The desktop application requires an account that can access the WeChat Official Account platform to scan in and establish a local login session. Ordinary personal WeChat login is outside the desktop scope because target account search and article-list synchronization depend on the official account platform session and token.

**Consequences**

- Login copy, onboarding, and errors should explicitly say "微信公众号后台账号" rather than generic "微信扫码".
- The Rust backend must preserve the current official account platform login flow: login session, QR code, scan polling, biz login, token extraction, and local cookie storage.
