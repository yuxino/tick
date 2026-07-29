# DeepSeek Settings Design

Tick will manage the user's DeepSeek API key entirely inside the application. The key will be written to macOS Keychain under the service `com.gavin.tick` and account `deepseek-api-key`; it will never be stored in localStorage, JSON, plist, source code, logs, or GitHub Actions. The React UI receives only configuration status and a short masked hint. It never receives the saved secret.

A compact settings button will sit beside “新建任务” in the normal macOS toolbar. Opening it shows an “AI 服务” modal with the provider, masked configuration status, a password field for replacement, and actions to save, test, or remove the key. Saving and removal happen through Tauri commands implemented in Rust. “Test connection” sends a minimal authenticated request from Rust and returns only success or a sanitized error.

AI script generation reads only from Keychain. The existing environment-variable and login-shell fallback will be removed. If no key is configured, generation fails with an actionable message directing the user to “设置 → AI 服务”. Key input is trimmed, length-limited, and never included in errors.

The oversized header will be corrected at the same time: the mascot becomes a compact 36–40 px mark, the title/subtitle use normal toolbar scale, and the header returns to a fixed 72–76 px height. The design remains quiet and native-feeling rather than promotional.

Verification covers Keychain status/save/delete behavior at the Rust boundary, frontend type checking and build, existing Rust tests, and visual inspection of the rebuilt macOS app.
