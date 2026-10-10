# Recording Tools

Use VHS as the single terminal recording tool for ratatui-kit demos.

- `vhs` records declarative terminal sessions into GIF/MP4/WebM assets.
- A `.tape` file is the source of truth for regenerating the demo.

Official references:

- VHS: https://github.com/charmbracelet/vhs

## Install

On macOS with Homebrew:

```bash
brew install vhs
```

Notes:

- Homebrew's `vhs` formula installs the main VHS package and its declared dependencies.
- VHS may require a browser backend for video/GIF rendering; use the formula defaults first and report environment blockers plainly.

## Check Tools

```bash
command -v vhs
vhs --version
```

## VHS Pattern

Create `docs/tapes/<example>.tape`:

```text
Output docs/public/recordings/<example>.gif
Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""
Type "cargo run --example <example>"
Enter
Sleep 3s
Ctrl+C
```

Then run:

```bash
vhs docs/tapes/<example>.tape
```

Always set the terminal color environment in the tape. Codex and CI shells may
export `TERM=dumb` or `NO_COLOR=1`; ratatui/crossterm will then suppress style
codes even though a real local terminal shows colors. Keep the `Set` directives
before `Env`; VHS ignores late `Set Width` / `Set Theme` directives after
non-setting commands.

Use GIF for most docs pages. Use MP4 or WebM when the docs page or homepage needs video playback; change only the `Output` extension in the tape.

## VHS Screenshot Verification

VHS can write PNG snapshots from the same tape pipeline. Add `Screenshot <file.png>`
after any step to capture the terminal frame at that exact moment:

```text
Screenshot target/ratatui-docs-demo-loop/_verify/<example>/stable.png
```

A tape can keep its normal `Output docs/public/recordings/<example>.gif` and also
contain multiple `Screenshot` commands. Screenshots do not replace or affect the
GIF/MP4/WebM render; they are extra PNG files for visual verification.

Minimal pattern:

```text
Output docs/public/recordings/<example>.gif
Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""

Type "cargo run --quiet --example <example>"
Enter
Sleep 3s
Screenshot target/ratatui-docs-demo-loop/_verify/<example>/01-stable.png
Type "j"
Sleep 700ms
Screenshot target/ratatui-docs-demo-loop/_verify/<example>/02-after-interaction.png
Type "q"
Sleep 500ms
```

## Practical Fallbacks

### Mouse-driven examples

VHS keyboard input alone does not exercise pointer handlers. For `mouse`, build
first with `cargo build --example mouse`, then run `vhs docs/tapes/mouse.tape`.
The tape launches `docs/scripts/mouse-demo.py`: a PTY runs the real binary,
inherits the recording terminal size, and receives timed SGR press, release,
drag and movement reports. Its actual output is relayed to VHS.

Keep compilation outside this capture: a fixed sleep may otherwise type the
next command into an unfinished build and expose shell output in the recording.
Verify the button count, divider movement, click count, modal and hover states
with screenshots. When changing layout, update the script's cell coordinates
and inspect the resulting states; a successful VHS exit alone does not prove
that the mouse events hit their intended targets.

If recording a full-screen TUI is flaky in CI or a headless shell, still leave useful test output:

- a tape file
- a command transcript
- tool versions
- notes describing the blocker

Do not replace a failed real recording with fake terminal art. The whole point of this loop is to build trustable runtime assets.
