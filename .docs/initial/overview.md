Overview

Build a minimal Rust CLI that exposes a single Review command. It accepts --model, --reasoning-effort, and --api-key (with OPENAI_API_KEY fallback), plus --default-branch, --additional-prompt, and --dry-run. The CLI computes a git diff against the merge base, gathers the list of changed files, and sends that to an LLM for review.

The system prompt is read from prompt.txt. The user prompt includes the diff and the list of touched files, but omits full file contents. Instead, the model can call two native tools: search_files (regex search across the repo) and read_file (slice reads with line numbers). The prompt will explicitly instruct the model to be judicious and avoid reading the entire codebase.

The OpenAI request uses the Responses API with tool calling. The CLI will loop, executing tool calls locally and returning results to the model until it emits the final JSON review response.
