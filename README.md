# blart

A minimal Rust CLI for AI-powered code review. It sends your git diff to an LLM and returns structured JSON feedback on typos, algorithmic errors, and obvious inconsistencies.

## What it does

**blart** reviews code changes by:
1. Computing a diff against your merge base (e.g., `main`)
2. Sending the diff and touched file list to an LLM (via OpenAI-compatible APIs)
3. Allowing the model to call `read_file` and `search_files` tools to inspect the code - this allows it to query for enough context to give a thorough review
4. Returning a JSON response with any substantive issues, and reasoning for its review

The system prompt instructs the model to focus on issues a human reviewer would catch but a compiler might miss—such as off-by-one errors, incorrect library usage, or contradictions between code and documentation.

## Usage

```bash
# Basic usage (requires OPENAI_API_KEY env var)
blart review

# Specify model and reasoning effort
blart review --model openai/gpt-4 --reasoning-effort high

# Dry run to see prompts without API call
blart review --dry-run

# Compare against a different base branch
blart review --default-branch develop

# Add extra context to the prompt
blart review --additional-prompt "Focus on security issues"
```

### Flags

- `--model` (default: `gpt-5.2-2025-12-11`): OpenAI-compatible model to use
- `--reasoning-effort` (default: `high`): One of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`
- `--api-key`: OpenAI API key (falls back to `OPENAI_API_KEY` env var)
- `--default-branch` (default: `main`): Branch to diff against
- `--additional-prompt`: Extra instructions for the reviewer
- `--dry-run`: Print prompts and exit without calling the API

### Environment variables

- `OPENAI_API_KEY`: API key for OpenAI or OpenAI-compatible providers
- `OPENAI_BASE_URL`: Override the base URL (e.g., for OpenRouter or local providers)

## How it works

**blart** is inspired by [robocop](https://github.com/simon-bourne/robocop).

Instead of sending full file contents upfront, it gives the model two tools:
- **`read_file`**: Read a file with line numbers (supports slice mode and indentation-aware extraction)
- **`search_files`**: Regex search across the repo with context lines

This keeps context sizes small and encourages the model to be judicious about what it reads.

## Output

blart simply prints the model's JSON response to stdout:

```json
{
  "reasoning": "The diff adds a new feature...",
  "substantiveComments": false,
  "summary": "n/a"
}
```

If `substantiveComments` is `true`, the `summary` field contains a human-readable list of issues (formatted in GitHub Flavored Markdown).

## License

MIT (see `LICENSE`)
