# Quality Gates

Use validation proportional to the change.

## Always Check

```bash
test -s docs/public/recordings/<example>.gif || \
test -s docs/public/recordings/<example>.mp4 || \
test -s docs/public/recordings/<example>.webm
test -s docs/tapes/<example>.tape
```

Record the actual output of:

```bash
cargo run --example <example>
```

For long-running TUI examples, use a bounded command such as:

```bash
timeout 5s cargo run --example <example>
```

On macOS, where GNU `timeout` may not exist, prefer a VHS tape that sends `Ctrl+C`.

## Docs Validation

If docs files changed:

```bash
cd docs
pnpm install --frozen-lockfile
pnpm build
```

If dependencies are already installed, skip install and run only `pnpm build`.

## Rust Validation

If Rust code or examples changed:

```bash
cargo fmt --all --check
cargo test --locked --all-features --workspace --lib --tests --examples
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

If only docs assets changed, full Rust validation is optional. Still build or run the selected example if the asset claims to represent it.

## Visual Quality

A runtime asset should be:

- readable at docs width
- free of local machine secrets or noisy prompts
- focused on the selected example
- reproducible from a VHS tape

Verify the recording visually with VHS `Screenshot`, not by extracting GIF frames
with ffmpeg.

1. Add `Screenshot` lines at the moments that prove the recording:
   - after the first stable frame, once compile output and shell prompts are gone
   - after each important interaction, mode change, or state transition
   - before exit when the final state matters
2. Write verification PNGs to an ignored scratch path, for example:

   ```text
   target/ratatui-docs-demo-loop/_verify/<example>/01-stable.png
   target/ratatui-docs-demo-loop/_verify/<example>/02-after-interaction.png
   ```

   The repository ignores `target/`, so these PNGs cannot be accidentally staged.
   The committed assets remain only:

   ```text
   docs/public/recordings/<example>.gif
   docs/tapes/<example>.tape
   ```

3. Choose one verification style:
   - Temporary tape edits: add `Screenshot` lines to `docs/tapes/<example>.tape`,
     run `vhs`, inspect the PNGs, then remove the screenshot lines before commit.
     This is quickest when you are already changing that tape.
   - Verification copy: create a scratch copy under `target/ratatui-docs-demo-loop/`,
     add only `Screenshot` lines there, and run `vhs` on the copy. This is best
     when the committed tape is already correct and you only need visual proof.
4. Inspect the PNGs before accepting the GIF:
   - colors are present and readable
   - no compile logs, shell prompts, usernames, hostnames, or local paths are visible
   - the selected example is framed at the intended terminal size
   - interaction screenshots show the state changes claimed by the docs

If any historical note or local workflow suggests ffmpeg frame extraction for this
skill, remove it from the process. VHS `Screenshot` is the standard visual check.

If VHS cannot record the TUI reliably, report the limitation and leave the tape in the test output for the next iteration.
