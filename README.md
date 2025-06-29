# tsukimi
A simple third-party Jellyfin client for Linux.

[![Telegram](https://img.shields.io/badge/-Telegram_Group-red?color=blue&logo=telegram&logoColor=white)](https://t.me/tsukimi_frying_crab) ![Commit Activity](https://img.shields.io/github/commit-activity/m/tsukinaha/Tsukimi/main) ![Top Language](https://img.shields.io/github/languages/top/tsukinaha/Tsukimi) ![Github License](https://img.shields.io/github/license/tsukinaha/Tsukimi) [![Linux CI](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml/badge.svg)](https://github.com/tsukinaha/tsukimi/actions/workflows/build_linux.yml) [![Flathub Version](https://img.shields.io/flathub/v/moe.tsuna.tsukimi?color=white)](https://flathub.org/apps/details/moe.tsuna.tsukimi) [![Wiki](https://img.shields.io/badge/-Wiki-red?color=blue&logo=github&logoColor=white)](https://github.com/tsukinaha/tsukimi/wiki)


![Alt](https://repobeats.axiom.co/api/embed/82b1088ba840d89c50e1b5b3c1c40f4575b321a6.svg "Repobeats analytics image")

## About
A simple third-party Jellyfin client written in GTK4-RS, uses MPV as the video player, and GStreamer as the music player.

It’s also partially compatible with Emby.    
All of basic functions and most of admin functions are supported.    

You can play music and albums, as well as videos with more advanced settings.

## Screenshots
<div align="center">
 <img src="./docs/tsukimi1.png"/>
</div>
<div align="center">
 <img src="./docs/tsukimi2.png"/>
</div>
<div align="center">
 <img src="./docs/tsukimi3.png"/>
</div>

## Installation

<a href="https://flathub.org/apps/details/moe.tsuna.tsukimi">
<img
    src="https://flathub.org/assets/badges/flathub-badge-i-en.svg"
    alt="Download Tsukimi on Flathub"
    width="240px"
    height="80px"
/>
</a>

### Native Packages
[![Packaging status](https://repology.org/badge/vertical-allrepos/tsukimi.svg)](https://repology.org/project/tsukimi/versions)

### Arch Linux
```bash
# AUR release (https://aur.archlinux.org/packages/tsukimi-bin)
paru -S tsukimi-bin

# AUR latest commit (https://aur.archlinux.org/packages/tsukimi-git)
paru -S tsukimi-git

# archlinuxcn repo https://github.com/archlinuxcn/repo/blob/master/archlinuxcn/tsukimi-git/PKGBUILD
sudo pacman -Syu tsukimi-git
```

### AOSC OS
```bash
sudo oma install tsukimi
```

### Gentoo Linux
```bash
sudo eselect repository enable gentoo-zh
sudo emerge --sync gentoo-zh
sudo emerge --ask media-video/tsukimi
```

### Nix
`tsukimi` is available in nixpkgs since 24.11. 

### Source code
[Build on Linux](./docs/build_on_linux.md)

## MPV Config
[MPV-manual#files](https://mpv.io/manual/master/#files) 

## Contributing

### Translations

If you'd like to help translating **Tsukimi** into your language, please head over to [Weblate](https://hosted.weblate.org/projects/tsukimi/).   

<a href="https://hosted.weblate.org/engage/tsukimi/">
<img src="https://hosted.weblate.org/widget/tsukimi/main/multi-auto.svg" alt="翻译状态" />
</a>

## Disclaimer
The developers of this application does not have any affiliation with the content providers available.

## License
tsukimi is licensed under the [GPLv3](https://www.gnu.org/licenses/gpl-3.0.html) license.

Thanks to open-source projects like GNOME Music, Fractal, and Clapper—we referenced a lot from them during development, and everyone who contributed code or translations!

### Credits
- [GTK4-RS](https://github.com/gtk-rs/gtk4-rs)
- [MPV](https://github.com/mpv-player/mpv)
- [Adwaita](https://gitlab.gnome.org/GNOME/libadwaita/)

<sup>
Fonts used in screenshots: <a href="https://github.com/lxgw/LxgwWenKai-Screen">LXGW WenKai Screen</a> 
<br />
Anime in screenshots: <a href="https://www.fate-zero.jp/">Fate/Zero</a>
</sup>

