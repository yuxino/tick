# AI Automation Design

Tick's AI entry creates a complete, reviewable LaunchAgent draft instead of returning an isolated code snippet.

## Decisions

- DeepSeek returns strict JSON containing a `LaunchdJobInput`, a plain-language summary, and a list of risks.
- Rust deserializes and validates the draft with the same validation used for manually created tasks.
- Generated execution is limited to inline Node.js using built-in modules.
- AI output never saves, enables, or runs a LaunchAgent automatically.
- The existing task form remains the review surface, including script editing and one-off debug runs.
- File deletion should use the Trash, and file, network, or private-directory access must be called out in `risks`.

## API key storage

The API key is stored in the application's platform configuration directory under `com.gavin.tick/settings.json`. On Unix-like systems, Tick sets the directory to `0700` and the file to `0600`. The key is persistent but not encrypted. The UI and documentation say this directly.

This is intentionally simpler and more portable than platform-specific credential stores. The Rust backend never returns the full saved key to React.

## Failure handling

- Missing or malformed AI JSON is rejected with a retryable user-facing error.
- Drafts that fail Tick's job validation are rejected.
- Unsupported execution modes are rejected.
- DeepSeek network and HTTP errors remain visible without exposing the saved key.
