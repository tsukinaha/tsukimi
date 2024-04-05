# Tsukimi       

  ![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main)
  ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi)
  ![GitHub License](https://img.shields.io/github/license/tsukinaha/tsukimi)

  
使用 GTK4-RS 编写的第三方 Emby 客户端              
For Linux ~~Only~~.


  <a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml">
    <img alt="Linux CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg"/>
  </a>
  <a href="https://github.com/tsukinaha/tsukimi/actions/workflows/build_gnu.yml">
    <img alt="Windows GNU CI status" src="https://github.com/tsukinaha/tsukimi/actions/workflows/build_gnu.yml/badge.svg"/>
  </a>

  ![AUR Version](https://img.shields.io/aur/version/tsukimi-git)

## Build
请见 [Dockerfile](https://github.com/tsukinaha/tsukimi/blob/main/Dockerfile)

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
如果没有配置，则读取环境变量`$MPV_HOME`，都没有则保持libmpv默认行为

**具体配置请见:[MPV-manual#files](https://mpv.io/manual/master/#files)**

## Credits
- [gtk4-rs](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)
