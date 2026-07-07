# streamsmith demo tape — renders the hero GIF.
#
#   go install github.com/charmbracelet/vhs@latest   # or: brew install vhs
#   cargo install --path .                            # put `streamsmith` on PATH
#   vhs demo.tape                                      # writes docs/demo.gif
#
# vhs docs: https://github.com/charmbracelet/vhs

Output docs/demo.gif
Require streamsmith
Require ffmpeg

Set Shell "bash"
Set FontSize 15
Set Width 1120
Set Height 660
Set Padding 20
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set TypingSpeed 55ms

# --- hidden setup: ensure PATH, and make a throwaway sample clip ---
Hide
Type `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"`
Enter
Type `ffmpeg -f lavfi -i testsrc2=size=1920x1080:rate=30 -f lavfi -i sine=frequency=440 -t 8 -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest talk.mp4 -y -loglevel error`
Enter
Sleep 6s
Type "clear"
Enter
Show

# --- the demo: one command → a web-ready adaptive stream ---
Sleep 700ms
Type "streamsmith talk.mp4 --no-open"
Sleep 600ms
Enter
Sleep 9s
Ctrl+C
Sleep 1500ms
