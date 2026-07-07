# Launch posts (draft)

Not committed marketing — this file is a scratchpad for launch day. Delete it
before publishing if you'd rather not ship it in the repo.

**Before you post anything:**
1. ~~Replace every `yourname`~~ — done (set to `Amahdip`). Add a name/email to `authors` in Cargo.toml if you want.
2. `cargo publish` so `cargo install streamsmith` actually works.
3. Record the GIF: `vhs demo.tape` → `docs/demo.gif`, then point the README hero at it.
4. Take one screenshot of the browser preview player and add it to the README too.
5. Push, then cut a release: `git tag v0.1.0 && git push --tags` (the release workflow builds binaries).

Post mid-morning US Eastern on a Tue–Thu for the widest reach. Reply to every
early comment — engagement in the first hour is what drives a post.

---

## r/rust

**Title:** `streamsmith: one command to turn any video into an adaptive HLS stream (my first Rust project)`

**Body:**

> I've been learning Rust and wanted a real tool at the end of it, not a toy. So
> I built **streamsmith**: point it at a video and it produces a ready-to-serve
> HLS adaptive-bitrate bundle — multiple quality levels, segmented,
> keyframe-aligned, with a correct master playlist — then serves a preview player
> so you can watch it adapt live. One command.
>
> ```
> streamsmith talk.mp4
> ```
>
> The thing that always stopped me doing this by hand is the wall of FFmpeg flags
> you copy-paste and never fully trust. streamsmith probes the source, plans a
> sensible ladder (never upscaling, widths kept to the source aspect ratio), and
> drives FFmpeg for you.
>
> It's a single ~670 KB binary, only depends on FFmpeg at runtime, and encodes
> the ladder rungs in parallel with bounded concurrency so it saturates the CPU
> without oversubscribing it. No async runtime — just threads and `std::process`.
>
> Being my first real Rust project, I'd genuinely value feedback on the code —
> the FFmpeg command construction in `src/package.rs` and the tiny preview server
> in `src/serve.rs` especially. Repo: <link>
>
> Roadmap has DASH output and hardware-accelerated encoders (NVENC/VideoToolbox)
> if anyone wants a good first issue.

*(r/rust likes humility + a real ask for code review. Lead with "my first
project" — the community is generous to first-timers who show working code.)*

---

## r/selfhosted

**Title:** `Made a tiny CLI that turns any video into a self-hostable adaptive stream in one command`

**Body:**

> If you've ever wanted to serve your own videos properly — adaptive quality that
> switches with the viewer's bandwidth, like the big platforms — but didn't want
> to hand-write FFmpeg HLS commands, I made **streamsmith** for exactly that.
>
> ```
> streamsmith lecture.mp4
> ```
>
> You get a folder of HLS segments + playlists you can drop behind nginx / Caddy
> / an S3 bucket and it just streams. It also spins up a preview player so you can
> check it before you deploy. Single binary, FFmpeg is the only dependency.
>
> Screenshot of the preview + a quick demo in the README: <link>
>
> It's early (v0.1) and open source (MIT/Apache). Feedback and feature requests
> very welcome.

*(r/selfhosted cares about "I can host this myself with tools I already run."
Emphasize the static output + nginx/Caddy/S3, not the Rust internals.)*

---

## Show HN (optional, higher variance)

**Title:** `Show HN: Streamsmith – one command to turn a video into an adaptive HLS stream`

**Body:** 2–3 sentences: what it does, the one-liner, that it's a single binary
needing only FFmpeg, and a direct link. HN prefers terse. Post it yourself (Show
HN must be by the author) and be around to answer questions.

---

## The one-line pitch (reuse everywhere)

> Turn any video into a ready-to-serve HLS adaptive-bitrate stream — with a
> built-in preview player — in one command.
