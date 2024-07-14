<h1 align="center">Tsukimi</h1>

> A Simple Third-party Emby client.    
> 使用 GTK4-RS 编写的第三方 Emby 客户端

![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main) ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi) ![Github License](https://img.shields.io/github/license/tsukinaha/Tsukimi) [![Linux CI](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml) [![Windows GNU CI status](https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml) [![AUR Version](https://img.shields.io/aur/version/tsukimi-git)](https://aur.archlinux.org/packages/tsukimi-git) 

## Notice

> [!IMPORTANT]
> READ BEFORE INSTALLATION    

- **对 Windows 的支持不做任何保证**
- Trello Kanban     https://trello.com/b/8GOsuygJ/%E5%95%8A%EF%BC%9F    

## Screenshots
<div align="center">
 <img src="./docs/tsukimi.png"/>
</div>

## Build
### Linux
- 请见 [Dockerfile](https://github.com/tsukinaha/tsukimi/blob/main/Dockerfile)

## Installation
### Linux
Archlinux
```
# From AUR
paru -S tsukimi-git
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
- Linux: `$XDG_CONFIG_HOME/mpv`
- Windows: 
```
|__bin\
|__share\
|__lib\
|__mpv\
|    |__mpv.conf
|    |__input.conf
|    |__scripts\
|    |    |__ .......
|    |__ .......
|__config\
```
Priority:
`./mpv`>`$MPV_HOME`>`%APPDATA%/mpv`
[MPV-manual#files](https://mpv.io/manual/master/#files) 


## Themes

- 在自定义样式表时请使用 Default [更多](https://wiki.archlinux.org/title/GTK#Configuration)

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

