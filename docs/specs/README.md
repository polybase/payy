# Specs

- `docs/specs/` contains the complete durable specification for shipped features.
- Should remain consistent with implementation at all times
- Keep `docs/specs/` complete, even when some material is also presented elsewhere - linking to impl README.md is acceptable. Linking outside of docs/ is also acceptable. You can move material to respective crates/packages when it makes sense.
- Split large specs across multiple files so each spec file stays under 250 lines.
- Keep public-facing documentation in `docs/public/`, even if it is narrower or more selective than the full specification.
- Duplication between `docs/specs/` and `docs/public/` is acceptable when needed, because `docs/specs/` must remain the complete spec.
- Specs are authored as `.mdx` files.
- After making changes under `docs/specs/`, run `node docs/tools/spec-lint docs/specs/* docs/specs-wip/*`.
- See `docs/dev/tools/spec-lint.md` for frontmatter, MDX tags, and code-anchor conventions.
