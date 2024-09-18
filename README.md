![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main) ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi) ![Github License](https://img.shields.io/github/license/tsukinaha/Tsukimi) [![Linux CI](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml) [![AUR Version](https://img.shields.io/aur/version/tsukimi-git)](https://aur.archlinux.org/packages/tsukimi-git) [![Static Badge](https://img.shields.io/badge/Telegram_Group-blue)](https://t.me/tsukimi_chat)


## Tsukimi
Tsukimi is a simple third-party Emby client written in GTK4-RS, uses MPV as the video player, and GStreamer as the music player

## Frequently Asked Questions
https://dev.tsukinaha.org/tsukimi

## Screenshots
<div align="center">
 <img src="./docs/tsukimi1.png"/>
</div>
<div align="center">
 <img src="./docs/tsukimi2.png"/>
</div>

## Build
[Build On Linux](./docs/build_on_linux.md)

## Installation
### Linux
Archlinux
```
# From AUR
## Git Version
paru -S tsukimi-git

## Bin
paru -S tsukimi-bin
```
 

### Windows
- Scoop
```
# Install tsukimi only, tsukimi included in Scoop/Extras
## if you want to tansfer previous installed tsukimi from scol to Extras, first uninstall it and reinstall by `scoop install Extras/tsukimi` 
scoop install tsukimi

# Or install tsukimi with mpv configs
scoop bucket add scol https://github.com/Kosette/scol.git
scoop install tsukimi-full
```
- [Release](https://github.com/tsukinaha/tsukimi/releases/latest)

## MPV Config
Set config=true to enable external config.
- Linux: `$XDG_CONFIG_HOME/mpv`
- Windows: `$MPV_HOME`>`%APPDATA%/mpv`

[MPV-manual#files](https://mpv.io/manual/master/#files) 

## Credits
- [gtk4-rs](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)

## Thanks
- Gnome Music
- Fractal
- Clapper

## Star History

<a href="https://star-history.com/#tsukinaha/tsukimi&Timeline">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=tsukinaha/tsukimi&type=Timeline&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=tsukinaha/tsukimi&type=Timeline" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=tsukinaha/tsukimi&type=Timeline" />
 </picture>
</a>

