# Tsukimi       

  ![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main)
  ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi)
  ![GitHub License](https://img.shields.io/github/license/tsukinaha/tsukimi)

  
使用 GTK4-RS 编写的第三方 Emby 客户端              
For Linux ~~Only~~.    
请注意 设置页第一项的功能是    
开启：每10s回报一次进度 播放与停止也回报    
关闭：每300s回报一次进度 播放与停止也回报    
如果你不能接受懒回报 请立刻删除此软件并拉黑    
只对 aur 用户的 issue/pr 积极回应

  <a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml">
    <img alt="Linux CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg"/>
  </a>
  <a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml">
    <img alt="Windows GNU CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_release.yml/badge.svg"/>
  </a>

  [![AUR Version](https://img.shields.io/aur/version/tsukimi-git)](https://aur.archlinux.org/packages/tsukimi-git)

## Build
请见 [Dockerfile](https://github.com/tsukinaha/tsukimi/blob/main/Dockerfile)

## Installation
### Windows
- Install via Scoop / 通过Scoop安装
```
scoop bucket add scol https://github.com/Kosette/scol.git
scoop install tsukimi
```

## MPV Config
- Linux: 读取默认配置 (`$XDG_CONFIG_HOME/mpv`)
- Windows: 
首先读取以下路径中的配置
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
如果未配置，则保持libmpv默认行为 (优先级：`tsukimi`>`$MPV_HOME`>`%APPDATA%/mpv`)

[Windows MPV Config懒人包](https://github.com/ZBound/mpv_config/raw/main/tsukimi-mpv-config.7z)

**具体配置请见:[MPV-manual#files](https://mpv.io/manual/master/#files)** 


## Themes

- 除内置的6个主题外提供了跟随系统的选项 以便切换深浅色 和尊重用户自定义 css 样式表 [了解详情](https://wiki.archlinux.org/title/GTK#Configuration)
- 主题来自 [Gradience](https://github.com/GradienceTeam/Gradience)

## Credits
- [gtk4-rs](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)
