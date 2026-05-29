# Reference Implementations

This directory contains reference implementations that inform the design of llm-usage.

## ccusage

The [ccusage](https://github.com/ryoppippi/ccusage) repository is cloned here as a reference for:

1. **LLM Pricing**: LiteLLM pricing fetch logic with daily caching
2. **Session Parsing**: How to parse JSONL session files from Claude, Codex, and OpenCode
3. **Token Calculation**: How to calculate token usage and costs

### Key Files Referenced

- `rust/crates/ccusage/src/pricing.rs` - LiteLLM pricing fetcher with fuzzy model matching
- `rust/crates/ccusage/src/cost.rs` - Cost calculation with tiered pricing
- `rust/crates/ccusage/src/adapter/claude/` - Claude JSONL parser
- `rust/crates/ccusage/src/adapter/codex/` - Codex JSONL parser
- `rust/crates/ccusage/src/adapter/opencode/` - OpenCode JSONL parser

### Implementation Notes

- Pricing is fetched from LiteLLM's GitHub repository
- Prices are cached locally and refreshed daily
- Model names are fuzzy-matched to handle provider prefixes and version suffixes
- Token usage is extracted from JSONL session files in each agent's data directory
