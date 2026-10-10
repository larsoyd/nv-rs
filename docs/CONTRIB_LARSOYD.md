# Contributor research from larsoyd (fork `larsoyd/nv-rs`)

Findings from playtesting nv-rs on Linux (KDE Plasma on Wayland, a 3440×1440 primary monitor)
on the fork branch `larsoyd/nv-rs`. They were made following [METHODOLOGY.md](METHODOLOGY.md)
and [RESEARCH_WORKFLOW.md](RESEARCH_WORKFLOW.md), but they have not been reviewed by the
maintainer. **Double-check every address and value before relying on it.**

## How these were made

- **Executable.** The GOG FalloutNV.exe 1.4.0.525, SHA-256
  `89e7020487e80f20ddc92ea641c4433b2c6839a82f45a076e414d167d11dd0fe`. The project's own research
  uses the Steamless-unpacked Steam exe (`19406942...739f`, [LEDGER.md](LEDGER.md)). The two are
  different files. Every address below matched the function that this repository's docs or code
  already describe at that address. Section hashes were not compared.
- **Ghidra.** Queried read-only over MCP: decompile, disassemble, xrefs, strings and byte search.
  Nothing was renamed, typed, commented or saved. The decompiler output left read-only data as
  addresses, and constants were read from the exe's file bytes.
- **Evidence classes**, kept apart as the ledger asks:
  - *traced*: read in Ghidra;
  - *observed*: seen in the original game by the user;
  - *tested*: a unit test that fails when the code is broken;
  - *played*: confirmed in the viewer by the user, with the viewer's log.

## Findings

### Mouse capture and the system pointer

- *Traced.* The mouse device's cooperative level is set at `00a22660`: 5 (exclusive and
  foreground), or 6 (non-exclusive and foreground) when `bBackground Mouse:Controls` is set. That
  setting is registered at `00fb44d0` and defaults to 0.
- *Traced.* The window procedure `0086a0a0` handles `WM_NCHITTEST`. Over the client area it calls
  `ShowCursor(0)` until the count goes negative, and over the frame it shows the cursor again. So
  the system pointer never shows over the game, menus included, and only the game's own cursor is
  drawn.
- In the viewer, the pointer is locked where the window system can lock it (Wayland). On Win32 and
  X11 it is confined and moved back to the middle each frame. In menus it is hidden and confined
  (`viewer/src/main.rs`: `pointer_hold`, `cursor_state`). *Tested* and *played*.

### The menu cursor's picture

- *Traced.* `0070b240` builds the cursor tile: depth 2000, 32 by 32, with red, green and blue at
  255 and alpha at 255.
  - Its FILENAME (trait `0xfcc`) comes from `sCursorFilename`, the setting object at `011d475c`
    registered at `00f6ccc0`.
  - Its TEXATLAS (trait `0xff9`) comes from `sDefaultTexAtlas`, the setting object at `011f3368`
    registered at `00fb33f0`, which defaults to `Interface\InterfaceShared.tai`.
- *Data.* That atlas maps the cursor's file name into `InterfaceShared0` at u 0.562988,
  v 0.812988, with width and height 0.030273 (31 by 31 texels): an amber arrow. The standalone
  cursor picture in `Fallout - Textures2` is a green arrow that the game never draws.
- *Observed.* The user's screenshot of the original shows the amber arrow. The viewer had left the
  atlas off and drew the green one. Fixed in `viewer/src/game_menus/mod.rs` (`load_cursor`).
  *Tested* and *played*.

### Window size and fullscreen

- *Traced.* `bFull Screen:Display` defaults to 1 (`00f3fd00`). `iSize W:Display` and
  `iSize H:Display` default to 640 and 480 (`00f3fc70`, `00f3fca0`).
- The viewer opens a window at `iSize`. When fullscreen it opens borderless at the monitor's own
  size instead of taking exclusive fullscreen at `iSize`. This is a deliberate improvement, not
  fidelity. *Tested* and *played*.
- *Not Ghidra.* On Linux the user's `Fallout.ini` and `FalloutPrefs.ini` are found in the Wine
  prefix that holds the install (`drive_c/users/<user>/Documents/My Games/FalloutNV`). Two users
  there with settings is ambiguous, so neither is used. Upstream PR #62 covers the same ground for
  Proton, and its version should replace this one once it merges.

- *Not Ghidra.* On Wayland, winit 0.30 has no primary monitor (`primary_monitor()` returns
  `None`), so `MonitorSelection::Primary` asks for fullscreen without naming an output, and KWin
  picks one itself. A later request that names an output does not move a window that is
  already fullscreen. WAYLAND_DEBUG showed `set_fullscreen(wl_output#7)` answered with the old
  2560x1440 configure. The viewer now reads KDE's `kde_output_order_v1` at startup (the first
  output is the desktop's primary), starts windowed, and makes its first fullscreen request name
  that output (`viewer/src/primary_output.rs`). Without the protocol (other desktops, X11) it
  behaves as before. *Tested* and *played*: KWin reported `output=DP-1 fullscreen=true
  frame=0,0 3440x1440` while the user was active on another monitor.

### Movie placement

- *Traced.* `00ec0cd0` places a movie through `00ec2aa0` (scale), `00ec2b20` (x) and
  `00ec2bb0` (y) on the movie player, the singleton at `0126fac4`, created at `00866ff0` with its
  vtable at `01082564`. Vtable `+0x28` is the width (`008672f0`), `+0x2c` the height
  (`00867340`) and `+0x34` the x offset (`00867390`).
  - Letterboxed movies are scaled to the width and centred vertically. The divisor is the
    double 2.0 at `01011590`.
- *Traced.* AMD Eyefinity at `004de100`: when a horizontal display group is active (flags at
  `011c70ed` and `011c70ee`), the movie width becomes the group's width ÷ 3.0 (the double at
  `01021928`) and the x offset becomes that same width. Movies then play on the middle panel only.
  A further gate at `01189484` has only a read xref, so what sets it is untraced.
- On one 21:9 monitor the original would crop a letterboxed movie top and bottom: 247.5 px of each
  edge of the 1280×720 intro at 3440×1440. The viewer fits it inside the screen with bars at the
  sides instead, which is a deliberate improvement in the spirit of the Eyefinity path. It also
  places the movie again whenever the window's size changes (`viewer/src/movie.rs`: `placement`,
  `fit_movie`). *Tested* and *played*.

### Drawing a weapon with nothing equipped

- *Traced.* Ready Item (`009466d0`) calls `008a6840(1)`, which is `SetWeaponOut`
  ([ANIMATION.md](ANIMATION.md)). An actor with no weapon equipped gets one chosen only when it is
  not the player (the player is the object at `011dea3c`). So the player raises fists.
- *Data.* In a start straight into a cell, `CaravanPack.esm`'s preorder script adds
  `WeapNVBinoculars`. The viewer drew the player's first carried weapon, which was those
  binoculars. Fixed in `world::combat::weapon_in_hand`, where only other actors fall back to a
  carried weapon. *Tested* (world `melee`, `npc_combat`) and *played*.

### The scope scissor

- *Traced.* With a scope up, `00870bd0` scissors the world to a centred band from
  `left = W × k` to `W − W × k`, where `k = InterfaceManager+0x4d0 × fScopeScissorAmount × 0.5`
  (the double 0.5 at `01011588`).
  - `fScopeScissorAmount:Display` is the setting object at `011dea8c`, registered at `00fa3170`,
    with its default float 0.3 at `01017870`.
  - `+0x4d0` is set at `00712e90` to the backbuffer's width ÷ height ÷ 4/3 (`01021790`, the float
    4/3 stored as a double). It is 1.0 in the constructor `0070a130`, which calls `00712e90`
    during init.
  - On Eyefinity, two black quads are drawn instead.
- [PLAYER_ACTIONS.md](PLAYER_ACTIONS.md) listed this scissor as not done because the frame covers
  the rest. That holds at 16:9, but not at 21:9: at 3440×1440 the band's edge sits at 45.6 units at
  the overlay's 380, just inside the frame's 46.
- The viewer draws black bars outside the band behind the overlay, sized from the HUD picture's
  real size. A guard remeasures the overlay camera whenever its size disagrees with the picture's
  (`viewer/src/scope.rs`: `scissor_margin`, `scissor_bars`, `refit_scope_camera`). *Tested* and
  *played*.

### The Vit-o-matic camera on wide screens

- *Traced.* `007945f0` builds the menu frustum as left/right `∓tan(fDefaultFOV × π/180 × 0.65) ×
  0.75` (doubles `01023128` and `01074b98`, float `01016264`), and top/bottom as the same times
  height ÷ width. `004dc1f0` returns the width (`0118947c`) and `004dc200` the height (`01189480`).
  `004da670` passes them to `CreateWindowExA` in that order. The width stays fixed and the height
  shrinks on wider screens.
  - The flag `011f9426` swaps in the requested height only on the letterbox fallback. That
    fallback is when fullscreen can't get the exact mode (`004dc710` sets `011c70ec`).
- *Data.* From the real `NV_VitoMaticVigorTester_Activate.NIF` at the Forward pose, the page
  buttons P1_LT and P1_RT reach down to tan −0.420 and P1_Decrease to −0.398. The frustum's
  vertical half-tangent is 0.481 at 16:9 but 0.358 at 3440×1440, so at 21:9 the original's
  camera leaves those buttons below the screen. The user saw this in play.
- The viewer keeps the 16:9 height on screens wider than 16:9 and widens the view instead. This is
  a deliberate improvement. At 16:9 and narrower nothing changes (`cellview::vigor::camera`, test
  `screens_wider_than_16_9_keep_the_16_9_height`). Lockpicking (`0078e900`) and the rendered
  terminal build the same width-fixed frustum and are unchanged.

## Open

- `00709d50` sets the overlay frustum only on the Eyefinity path (guarded by the flag at
  `011c70ed`). On a single screen the menu and HUD 3D root's frustum is the orthographic one from
  `00712e90`, set once at init. Which camera the scope node really uses, and whether the viewer's
  overlay projection matches it, is unverified.
- The GOG versus Steam exe: compare section hashes before treating these addresses as the
  project's.
- Upstream PR #91 (the Wayland feature) and #62 (Proton prefix settings) overlap this fork's
  Wayland and Wine-prefix changes. The fork adopts the upstream versions once they merge.
