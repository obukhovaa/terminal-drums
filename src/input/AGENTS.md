# AGENTS.md — Input Module

- Command parser uses `trim_start_matches('/')` (not `strip_prefix`) to handle `//play` from the `:` → `/` pre-fill + user typing `/play`.
- Prefix matching: if a partial command like `/pla` has exactly one matching command, it resolves to that command. Multiple matches → Unknown error.
- `arg_hint()` returns human-readable placeholder text per ArgSpec: `<number>`, `<name>`, or `easy | medium | hard`. Used by console widget for placeholder display.
- The 10ms poll timeout in the input thread does NOT affect timestamp accuracy (timestamp is taken at `read()` time). It only affects delivery delay to the game thread (~18ms worst case).
