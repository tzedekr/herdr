# Herdr Radar outline-selection candidate

Prepared locally against Herdr v0.9.0 (`b99002ac99b09e00b4ca692436cb15a6b0d676f1`), on `local/radar-outline-selection`.
**Rolled back at Cole's request on 2026-09-15. Original executable and pre-outline config restored; reload applied and all 36 current pane terminal IDs preserved.**

Historical activation record: The verified binary is now at `~/.local/bin/herdr`, with `[ui] agent_selection_style = "outline"`. All 31 original pane terminal IDs and the running server were preserved. The existing client still has its old renderer loaded. Rollback and activation evidence: `activation-20260915-110805`.

## Appearance

The opt-in mode draws accent-colored `[ … ]` brackets on the last nonblank line of the selected expanded agent entry, with no background fill. Radar's headings and blank spacer lines remain outside the marker. The selector stays one line high while the existing click target and row order stay intact.

This is a terminal-cell bracket outline, not a rectangular box requiring extra rows. A single column at the right edge is reserved in outline mode, whether focused or not. No fonts, palette, metadata, plugins, agent states, or protocol fields are changed. The Spaces list and collapsed sidebar retain their existing selection treatment.

The new key is outside Radar's managed config blocks, so Radar's sidebar regeneration will not remove it:

```toml
[ui]
agent_selection_style = "outline"
```

The existing default remains `"fill"`. `preview-config.toml` contains a task-local copy of the current config with only that new option added. Do not copy it wholesale over a newer live config.

## Preview

`selection-preview.png` is rasterized from synthetic cell buffers produced by the actual candidate renderer test. It is not a screenshot of the running app. The fixture uses Radar's heading and working-title colors, with the current pale-blue selection color for comparison. Live Ghostty access was denied by Computer Use earlier in this task, so live visual QA has not occurred.

## Verification

- Release build: passed.
- Candidate executable `config check` against isolated preview config: passed.
- Client-shell tests: 203 passed, including outline defaults/parsing, grouped headings/spacers, background preservation, Unicode and narrow clipping, stable title truncation, config reload, mouse hit targets, scrolling, and collapsed behavior.
- Python config-reference/architecture checks: 20 passed.
- Documentation tests: 7 passed.
- `cargo fmt --check`, `git diff --check`, config-reference validation: passed.
- Required render-scale profile: 2 passed. The focused client rendering micro-profile measured fill/outline at 2.36/2.21 microseconds for one agent and 19.84/20.11 for 15 agents. These are single-run supporting measurements, not guaranteed timings.
- Strict all-target Clippy is not clean: Rust 1.98 emits six diagnostics in unchanged upstream files (`pane_graphics.rs`, `ghostty/mod.rs`, `server/handoff.rs`, `terminal_theme.rs`, and `integration/tests.rs`). Those files are byte-identical to the base; see `clippy.log`. No allowances or unrelated fixes were added.
- Independent read-only renderer review: no blocking defect found.
- Full `just check`, Windows lint, and the full repository test suite were not run. This is a scoped local candidate, not an upstream release.

## Artifacts

- Patch: `outline-selection.patch`; SHA-256 `2a146f6a1637b3b24a0b25661c11066a58495438b36fc1d9f8c5acba4f097b91`.
- Binary: `/Users/colecarden/Projects/herdr-outline-selection/target/release/herdr`; SHA-256 `101c329f277d2a54233c45383dbda2043a515a30638a6e93dfcc6594ed0a46c3`.
- `manifest.json`: revision and hash record.
- Logs: `shell-tests.log`, `release-build.log`, `render-scale.log`, `clippy.log`, `docs-tests.log`, `preview.log`.

## Activation boundary

Cole approved session-preserving activation. The binary was atomically replaced and the new setting added without restarting or reloading the server. No commit, PR, publication, or server handoff was performed. Computer Use denied access to Ghostty, so Cole must press Ctrl+B then Q to detach the client, and run `herdr` from the outer shell to reconnect. Completion of that step and live appearance verification remain pending.
