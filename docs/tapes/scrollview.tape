Output docs/public/recordings/scrollview.gif

Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms

Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""

Type "cargo run --quiet --example scrollview"
Enter
Sleep 4s
Type "jjjj"
Sleep 900ms
PageDown
Sleep 1200ms
# b toggles Scrollbars::over_border: scrollbar on the block border ↔ inset inside it.
Type "b"
Sleep 1400ms
Type "b"
Sleep 1200ms
Type "kk"
Sleep 800ms
PageUp
Sleep 1000ms
Type "q"
Sleep 500ms
