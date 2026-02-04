# Getting started with A-Eye CLI

The default command is `a-eye`.

Quick start:

```bash
# Interactive UI
a-eye

# First-run wizard (model + safety tier)
a-eye setup

# Deterministic workflow path
a-eye scan
a-eye plan "describe your goal"
a-eye patch --from .nlpg/runs/<run_id>/plan.json
a-eye verify
```

Set up your model provider (commercial or open-source local):

```bash
# Show built-in presets
a-eye models list

# Open-source local default
a-eye models setup --provider ollama --model llama3.1:8b

# Commercial default
a-eye models setup --provider openrouter --model openai/gpt-oss-20b \
  --base-url https://openrouter.ai/api/v1 \
  --api-key-env OPENROUTER_API_KEY
```

You can also run `a-eye models setup` with no flags to use the interactive picker.

Set or update safety tier directly:

```bash
# Tier 1 = safest (plan + diff only)
a-eye setup --tier 1

# Tier 2 = supervised apply
a-eye setup --tier 2
```

For broader CLI feature docs, see [this documentation](https://developers.openai.com/codex/cli/features#running-in-interactive-mode).
