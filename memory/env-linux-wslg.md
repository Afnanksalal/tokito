# Env-specific: Linux / WSL2 / WSLg

> ⚠️ This file is scoped to a particular dev environment, not the project as a whole. Ignore it unless you are on WSL2/WSLg.

**Confirmed working on 2026-05-19** on WSL2 (Ubuntu, kernel 5.15 microsoft-standard-WSL2, WSLg present — `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir`).

**The command that works:**

```
WINIT_UNIX_BACKEND=x11 WAYLAND_DISPLAY="" LIBGL_ALWAYS_SOFTWARE=1 ./target/debug/tokito-native
```

**What fails (don't bother retrying without the env overrides above):**

- Plain `cargo run -p tokito-native` → eframe crashes with `winit EventLoopError: Exit Failure: 1`, preceded by `libEGL warning: failed to get driver name for fd -1` / `MESA: error: ZINK: failed to choose pdev` / `libEGL warning: egl: failed to create dri2 screen`. WSLg's Wayland path picks the zink GL→Vulkan adapter and falls over.
- `LIBGL_ALWAYS_SOFTWARE=1 WGPU_BACKEND=gl` **alone** is not enough — winit still tries Wayland first and fails with broken-pipe spam before any frame paints.

**Why the workaround:** with `WAYLAND_DISPLAY` cleared and `WINIT_UNIX_BACKEND=x11`, winit talks to the WSLg X server via `DISPLAY=:0`; `LIBGL_ALWAYS_SOFTWARE=1` makes Mesa use llvmpipe instead of the broken zink adapter.

**Known cosmetic issue:** under software GL the studio UI layout looks "off". Functionality works; this is a WSLg software-renderer artifact, not a code bug. Native Linux with a real GPU + Windows packaged build should not show this.

**First-launch side effect:** `tokito::db::embedded` downloads pg-embed Postgres 16 binaries into `~/.cache/pg-embed/linux/amd64/16.12.0/` (a few minutes; needs internet). Subsequent launches are instant.

**Apt deps that had to be installed manually** (the CI list minus what Ubuntu already has): `libxcb-shape0-dev`, `libxcb-xfixes0-dev`. The other 5 (`libgtk-3-dev libx11-dev libxcb-render0-dev libxkbcommon-dev libwayland-dev`) were already present.

**How to apply:**

- When running the desktop binary on WSL2, go straight to the X11+software-GL env vars; don't burn time on the default path.
- `scripts/run-linux.sh` (debug `--check` / `--release` / `--package`) is in place but **does not** set these env vars itself. Either export them in the shell first or extend the script.
- The cosmetic UI issue is **not** a bug to fix in code; flag any "fix the layout" requests as likely a real-GPU problem instead.
