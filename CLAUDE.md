# pov — Claude guidelines

## Skills

Use the `andrej-karpathy-skills:karpathy-guidelines` skill when writing, reviewing, or refactoring code.

## Python style

Follow the [Google Python Style Guide](https://google.github.io/styleguide/pyguide.html):
- Docstrings: one-line summary, blank line, then details if needed. Args/Returns/Raises sections for non-trivial functions.
- Type hints on all function signatures.
- `snake_case` for functions and variables, `PascalCase` for classes, `UPPER_CASE` for module-level constants.
- Max line length: 100 characters.
- Imports: stdlib → third-party → local, each group separated by a blank line.

## Dev best practices

- Make the smallest change that solves the problem. No speculative abstractions.
- No comments explaining what the code does — only why, when non-obvious.
- No dead code, no commented-out blocks, no TODO left in code without a matching entry in TODO.md.
- Prefer explicit over implicit. Trust the type system.
- No backwards-compatibility shims for code that isn't published or shared.

## Commits

One short subject line (imperative, no period). If more context is needed, add a blank line then a paragraph or bullet points — keep it concise.

```
add project card border color logic

- activity classification: this week / this month / older
- fallback to mtime when git log is unavailable
```

Commit regularly. Each commit should be self-contained: the codebase must be in a working state after every commit. Do not batch unrelated changes together.
