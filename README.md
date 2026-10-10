# larsoyd/nv-rs

My personal fork with changes to nv-rs that maintains strict upstream compatibility. The changes in this fork are done to make the software work better on my hardware, which is on a widescreen 4k monitor, Wayland, KDE Plasma and Linux. Any upstream fixes to my issues will be adopted instead. 

- **Mouse capture:** The mouse now stays inside the game window, and the system cursor no longer appears over the game or its menus.
- **Menu cursor:** Fixed the menu cursor using the wrong texture. It now shows the original amber cursor instead of the unused green one.
- **Windowing and fullscreen:** Improved fullscreen handling, especially on KDE Wayland, so the game opens on the correct monitor without flickering. Also added support for reading Fallout's existing INI settings from Wine/Proton installations.
- **Ultrawide cutscenes:** Movies now fit properly on ultrawide displays instead of having their tops and bottoms cut off. They also adjust automatically when the window is resized.
- **Unarmed combat:** Fixed the player pulling out random weapons from their inventory when nothing was equipped. Your character now correctly raises their fists instead.
- **Weapon scopes:** Fixed missing black borders around scoped views on ultrawide displays, particularly at 21:9.
- **Vit-o-matic Vigor Tester:** Fixed the character creation screen being cut off on ultrawide displays, making all its buttons visible without affecting standard resolutions.

Read more in [docs/CONTRIB_LARSOYD.md](docs/CONTRIB_LARSOYD.md)

---


# nv-rs

Upstream README [here](https://github.com/slaterain/nv-rs/blob/main/README.md).
