<h1 align="center">Tsukimi</h1>

> A Simple Third-party Emby client.
> 使用 GTK4-RS 编写的第三方 Emby 客户端

![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main) ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi) ![Github License](https://img.shields.io/github/license/tsukinaha/Tsukimi) [![Static Badge](https://img.shields.io/badge/Telegram-Group-blue)](https://t.me/+mhcPxx5kE6QzMjVl) [![Linux CI](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml) [![Windows GNU CI status](https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml) [![AUR Version](https://img.shields.io/aur/version/tsukimi-git)](https://aur.archlinux.org/packages/tsukimi-git) 

## Notice

> [!IMPORTANT]
> READ BEFORE INSTALLATION    

- 对 Windows 的支持不做任何保证，请善用 修改字体族/尺寸 的功能来弥补 Windows 可悲的字体渲染    
- 缩放导致的问题在 gtk4 无解，KDE Wayland/Windows 下请将系统缩放修改至 150% 以下来回避双重缩放    
- Windows 下如出现 窗口无法显示 黑边框 等情况
请尝试更新图形显示卡驱动 或安装 Archlinux    
- FR/Issue 前请检查是否重复    
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
- 如果你感觉组件过大（如图片），请设置此环境变量（X11 Only） [更多](https://wiki.archlinuxcn.org/wiki/HiDPI#GDK_3_(GTK+_3))    
```
$ export GDK_SCALE=1
```
- 如果你使用 KDE Wayland，组件大小将会是 `GDK 检测到的缩放大小 * KDE 系统缩放大小` 

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
- 主题来自 [Gradience](https://github.com/GradienceTeam/Gradience)

## Credits
- [gtk4-rs](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)

[![Star History Chart](https://api.star-history.com/svg?repos=tsukinaha/tsukimi&type=Timeline)](https://star-history.com/#tsukinaha/tsukimi&Timeline)
