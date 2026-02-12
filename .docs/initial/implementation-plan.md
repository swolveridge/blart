Implementation Plan

1) CLI shape and arguments
- Replace the current ad-hoc main with a clap Review command only.
- Flags:
  - --model (default from a const, aligned with robocop-core DEFAULT_MODEL)
  - --reasoning-effort (enum: none|minimal|low|medium|high|xhigh)
  - --api-key (optional; fallback to OPENAI_API_KEY)
  - --default-branch (default main)
  - --additional-prompt (optional; appended to the user prompt)
  - --dry-run (print prompts and exit without calling the API)

2) Git data collection (ported from robocop)
- Add a git module similar to robocop-cli get_git_diff and robocop-core git.rs.
- Capture:
  - diff against merge base
  - list of changed files
  - head hash, merge base hash, branch name, repo name, remote URL
- Return a GitData struct to keep the interface explicit and testable.

3) Prompt construction (ported from robocop-core)
- get_system_prompt(): read prompt.txt with include_str!.
- create_user_prompt(diff, files_changed, additional_prompt):
  - Start with a short instruction line.
  - Append additional_prompt if provided.
  - Include DIFF and the list of changed files.
  - Omit file contents; instruct the model to use tools for details.

4) Tool schemas (based on Roo-Code native tools)
- Define two tool schemas for the Responses API:
  - search_files: { path, regex, file_pattern }
  - read_file: { path, offset, limit }
- Keep schemas strict and minimal (slice mode only for read_file).
- Set conservative default limits (for example 2000 lines for read_file and max 50 matches for search_files) and cap output size.

5) Tool execution layer
- Implement search_files using walkdir + regex + optional globset.
- Implement read_file with UTF-8 only, line numbers, and offset/limit slicing.
- Return tool results in a consistent, machine-readable text format (with line numbers and file paths) so the model can cite specific locations.
- Enforce a hard cap on total tool calls per request (for example 8) to avoid runaway reads.

6) System prompt adjustment
- Prepend a short tool-use policy to the system prompt:
  - The model may call search_files and read_file.
  - It should start from the diff and file list.
  - It must be judicious and avoid reading large swaths of the repo.
- Preserve the existing JSON output requirements in prompt.txt.

7) Responses API integration
- Extend the existing OpenAI client DTOs to support:
  - tools definitions
  - tool calls in assistant responses
  - tool result messages
- Implement a tool-calling loop:
  - Send system + user prompt with tools.
  - If tool calls are returned, execute locally and append results.
  - Repeat until a final assistant message arrives.

8) Tests
- Add unit tests for:
  - read_file slicing, bounds, and UTF-8 handling
  - search_files regex matching and file_pattern filtering
  - create_user_prompt formatting with diff + file list + additional_prompt
- Avoid changing existing tests unless required by new behavior.

9) Output and UX
- If there is no diff or no changed files, print a short message and exit 0.
- If --dry-run is set, print system prompt, user prompt, and config, then exit 0.
- Otherwise print the final JSON review response exactly as returned by the model.
