# Contributing to streamsmith

Thanks for your interest — contributions are very welcome, from typo fixes to
new features on the [roadmap](README.md#roadmap).

## Getting started

```bash
git clone https://github.com/Amahdip/streamsmith
cd streamsmith
cargo build
cargo test
```

You'll need [FFmpeg](https://ffmpeg.org/download.html) installed to run the tool
itself (the unit tests do not require it).

Try it against a throwaway clip:

```bash
# generate a 6-second test video with ffmpeg
ffmpeg -f lavfi -i testsrc2=size=1920x1080:rate=30 -t 6 -pix_fmt yuv420p sample.mp4
cargo run -- sample.mp4
```

## Before you open a PR

The CI runs these three checks — please run them locally first:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

## Guidelines

- Keep the dependency footprint small — this is a single-binary tool and part of
  its appeal is that it stays lean. If a change adds a dependency, say why in the
  PR.
- New behavior should come with a unit test where it's practical (see
  `src/ladder.rs` and `src/probe.rs` for the style).
- FFmpeg command construction lives in `src/package.rs`; keep argument building
  pure and testable where you can.

## Reporting bugs

Open an issue with the input that triggered it (or how to generate one), the
exact command you ran, and the output. If it's an encoding failure, the tail of
FFmpeg's error that streamsmith prints is usually the key detail.
