Output docs/public/recordings/theme.gif

Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms

Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""

Type "cargo run --quiet --example theme"
Enter
Sleep 4s

# Cycle through default -> ocean -> sunset -> default.
Type "t"
Sleep 1200ms
Type "t"
Sleep 1200ms
Type "t"
Sleep 1200ms

Type "q"
Sleep 500ms
