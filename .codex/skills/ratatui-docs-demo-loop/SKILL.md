---
name: ratatui-docs-demo-loop
description: Use this skill whenever the user wants to turn a ratatui-kit example into a story-driven docs/tutorial page, create a real VHS terminal recording, automate example recording, build a docs homepage vertical slice, or run one complete docs/example loop for ratatui-kit. This skill should trigger even if the user says "just do one example", "record the demo", "make a tutorial from this example", "homepage needs runtime visuals", or "run the loop".
---

# ratatui Docs Demo Loop

Use this skill to produce one complete vertical slice for ratatui-kit documentation:

`example -> real VHS recording -> docs asset -> story-first tutorial -> optional homepage hook -> validation`

The goal is a repeatable loop, not a batch rewrite. A good run leaves behind one polished example that proves the pipeline works and can become the template for the rest.

## First Moves

1. Inspect the repository before making choices:
   - read `AGENTS.md` or provided repository instructions
   - inspect `Cargo.toml`, `examples/`, and the docs app (`docs/package.json`, `docs/astro.config.*`, existing docs pages)
   - check existing assets under `docs/public/` and example docs under `docs/src/content/docs/`
2. Pick exactly one example unless the user explicitly asks for more.
   - Prefer `counter` for the first loop because it is small, dynamic, and already tells a hooks/state story.
3. Use VHS as the only recording path.
   - default docs inline asset: `docs/public/recordings/<example>.gif`
   - homepage or richer video asset: `docs/public/recordings/<example>.mp4` or `.webm`
   - reproducible recipe: `docs/tapes/<example>.tape`
4. Keep generated assets real. Do not fake runtime output with hand-written terminal art or decorative mockups unless the user explicitly asks for concept art.
5. Keep edits scoped to the selected example, related docs page/component, and recording artifacts.

Read only the references needed for the current run:

- `references/recording-tools.md` when installing tools or recording terminal output.
- `references/artifact-contract.md` before writing generated assets or test outputs.
- `references/story-tutorial.md` before drafting or rewriting a tutorial page.
- `references/quality-gates.md` before final validation.

## Loop Workflow

### 1. Scope the slice

Capture these values in your working notes or test output:

- selected example name
- command used to run it
- VHS output format to produce (`gif`, `mp4`, or `webm`)
- docs page to update or create
- validation commands to run

If the user is exploring and has asked not to execute, stop after the plan. If the user asks to implement, proceed.

### 2. Prepare recording

Use deterministic terminal conditions:

- fixed terminal size, usually 80x24 or 100x30
- stable theme and font assumptions
- predictable waits
- no local secrets, hostnames, usernames, or shell prompts in the capture

Use a VHS tape for scripted input and waits so the asset can be regenerated.

### 3. Generate the artifact

Write assets to the docs public tree following `references/artifact-contract.md`.

For a first slice, produce one VHS-rendered asset:

- `docs/public/recordings/<example>.gif`
- `docs/public/recordings/<example>.mp4`
- `docs/public/recordings/<example>.webm`

Keep the VHS recipe under:

- `docs/tapes/<example>.tape`

### 4. Write the docs page

Use the story-first shape from `references/story-tutorial.md`:

- lead with the running example and its output
- show the smallest runnable code path
- explain the concept after the reader has seen the behavior
- connect the example to ratatui-kit architecture
- end with the next natural example

Avoid writing a marketing landing page as a tutorial. The tutorial should feel like a guided build, not a feature list.

### 5. Wire homepage only when useful

If the user asks for a homepage slice, reference the real runtime artifact from the selected example. The homepage may be visually ambitious, but the first viewport should still show actual product behavior.

### 6. Validate and report

Run the smallest meaningful validation for the slice, then broaden if risk justifies it:

- generated VHS artifact exists and is non-empty
- VHS tape exists and can reproduce the recording
- visual quality is checked with VHS `Screenshot` PNGs following `references/quality-gates.md`
- selected example still builds or runs
- docs build still passes
- full Cargo checks only when code changed or public Rust API behavior is touched

Summarize what changed, which commands ran, where the artifacts are, and what remains.

## Output Shape

When finishing a run, report:

- selected example
- generated artifact paths
- docs paths changed or created
- validation commands and results
- any tool limitations or fallback used

Keep the final response concise and concrete.
