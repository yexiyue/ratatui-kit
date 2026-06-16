Output docs/public/recordings/wrapped-text.gif

Set Width 1000
Set Height 600
Set FontSize 18
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms

Env TERM "xterm-256color"
Env COLORTERM "truecolor"
Env NO_COLOR ""

Type "cargo run --quiet --example wrapped_text"
Enter
Sleep 4s
Type "jjj"
Sleep 800ms
PageDown
Sleep 1000ms
Type "kk"
Sleep 800ms
PageUp
Sleep 1000ms
Type "q"
Sleep 500ms
