# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Connecting to MCP servers

A-Eye can connect to MCP servers configured in `~/.codex/config.toml`. See the configuration reference for the latest MCP server options:

- https://developers.openai.com/codex/config-reference

## Model setup

Use the guided model setup to configure commercial or open-source providers without editing TOML by hand:

```bash
a-eye setup
a-eye models list
a-eye models setup
```

Common non-interactive examples:

```bash
a-eye models setup --provider ollama --model llama3.1:8b
a-eye models setup --provider openrouter --model openai/gpt-oss-20b \
  --base-url https://openrouter.ai/api/v1 \
  --api-key-env OPENROUTER_API_KEY
```

Settings are persisted to `~/.codex/config.toml`.

The first-run wizard (`a-eye setup`) also writes:

- `approval_policy` (mapped from safety tier)
- `sandbox_mode` (mapped from safety tier)

## Apps (Connectors)

Use `$` in the composer to insert a ChatGPT connector; the popover lists accessible
apps. The `/apps` command lists available and installed apps. Connected apps appear first
and are labeled as connected; others are marked as can be installed.

## Notify

A-Eye can run a notification hook when the agent finishes a turn. See the configuration reference for the latest notification settings:

- https://developers.openai.com/codex/config-reference

## JSON Schema

The generated JSON Schema for `config.toml` lives at `codex-rs/core/config.schema.json`.

## Notices

A-Eye stores "do not show again" flags for some UI prompts under the `[notice]` table.

Ctrl+C/Ctrl+D quitting uses a ~1 second double-press hint (`ctrl + c again to quit`).
