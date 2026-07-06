Output docs/public/recordings/table.gif

Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms

Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""

Type "cargo run --quiet --example table"
Enter
Sleep 4s

# Wide + Grid: row 1 is selected by default (▶ gutter never clips the first column).
# l/h moves the column highlight; the cell pops at the row/column intersection.
Type "l"
Sleep 700ms
Type "l"
Sleep 900ms

# Compact density (c): Truncate + Outer border, so all rows and the footer fit at once.
Type "c"
Sleep 1400ms

# Move the selection: row + column highlight track together.
Type "j"
Sleep 450ms
Type "j"
Sleep 450ms
Type "j"
Sleep 700ms
Type "l"
Sleep 700ms
Type "l"
Sleep 900ms
Type "k"
Sleep 500ms
Type "h"
Sleep 900ms

# PageDown is not consumed by the page, so the ScrollView reveals the footer totals.
PageDown
Sleep 1300ms

# Back to wide density (Wrap + Grid).
Type "c"
Sleep 1300ms

# Empty state: e clears the rows (footer hides), then restores them.
Type "e"
Sleep 1300ms
Type "e"
Sleep 1100ms

Type "q"
Sleep 500ms
