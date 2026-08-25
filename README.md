# Eliza

A Rust TUI reimplementation of the classic Creative Computing **ELIZA** program
(`modern.bas`), with exactly the same conversational logic as the original
GW-BASIC source.

## Run

```bash
cargo run --release
```

- Type your problems after the green `?` prompt; press Enter to submit.
- `Esc` or `Ctrl+C` quits. (`shut` in any case wins the race and says
  `Shut up...` first, just like the original.)
- The conversation scrolls; the newest lines stay visible.

## Test

```bash
cargo test                 # 10 engine unit tests
printf 'hello\ni am sad\nshut up\n' | cargo run --example cli   # headless mode
```

## Files

- `src/lib.rs` — the Eliza engine, a byte-for-byte port of the BASIC logic:
  keyword priority search, apostrophe stripping, case-sensitive `SHUT`
  detection, repeat-input check, conjugation pairs, and per-keyword round-robin
  replies (including the original's quirks, e.g. the `!`-marker conjugation).
- `src/main.rs` — ratatui front end: red title block at the original
  `TAB(37/31/29)` offsets, green input line, white Eliza text.
- `examples/cli.rs` — stdin→stdout mode for scripting and diffing against the
  original interpreter.
- `modern.bas` — the original program being reimplemented.
