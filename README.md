       
# Tsukimi
> A Simple Third-party Emby client.

<a>
<img alt="Commit Activity" src="https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main" />
<img alt="Top Language" src="https://img.shields.io/github/languages/top/tsukinaha/Tsukimi"/>
<img alt="Github License" src="https://img.shields.io/github/license/tsukinaha/Tsukimi" />
</a>
  
使用 GTK4-RS 编写的第三方 Emby 客户端


<a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml">
<img alt="Linux CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg"/>
</a>
<a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml">
<img alt="Windows GNU CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml/badge.svg"/>
</a>
<a href="https://aur.archlinux.org/packages/tsukimi-git">
<img alt="AUR Version" src="https://img.shields.io/aur/version/tsukimi-git" />
</a>

## I want feature of ○○○!
欢迎提出你想要的功能    
如果它实用或有趣  我相信你很快就可以看到它

## Screenshots
<div align="center">
 <img src="./docs/tsukimi.png"/>
</div>

## Build
### Linux
- 请见 [Dockerfile](https://github.com/tsukinaha/tsukimi/blob/main/Dockerfile)

## Installation
### Linux
From AUR
`paru -S tsukimi-git`
 

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

## Tips
如果你感觉组件过大（如图片），请设置此环境变量（X11 Only） [更多](https://wiki.archlinuxcn.org/wiki/HiDPI#GDK_3_(GTK+_3))    
如果你使用 KDE Wayland，组件大小将会是 `GDK 检测到的缩放大小 * KDE 系统缩放大小` 
```
$ export GDK_SCALE=1
```
[Group](https://t.me/+8Sktkd13w00wNGNl)
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

- 在自定义样式表时请使用 Default [关于自定义样式表](https://wiki.archlinux.org/title/GTK#Configuration)
- 主题来自 [Gradience](https://github.com/GradienceTeam/Gradience)

## Credits
- [gtk4-rs](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)
