## 0.6.0

- 重写了一些组件
- 重写了一些方法
- 重写了加载进度条
- 重写了网络请求 现在 url 可以被自动格式化了 但仍需添加协议头
- 重写了错误处理
- 将电影与系列页面整合
- 现在 include 行 显示在剧集列表下方
- 添加了 附加内容 行
- 添加了 列表项图片 的加载 spinner 及无图片提示
- 为音乐页添加推荐行与同艺术家作品行
- 缩小 已看过 的标记大小
- 将列表项的第一行字体变粗 缩小列表项的字体
- 根页面的 stack 动画改为 OVER_LEFT_RIGHT
- 无专辑的单曲现在可以被显示了
- 主页的服务器列现在随外部同步
- 喜欢页面不再使用缓存
- 无内容的列表将不可见
- 从集进入项目页面时标题将会使用项目标题
- 设置窗口中可以搜索设置项了
- 增加了网络请求的 tracing
- 项目页中的 tags/studios/genres 可以进入对应的页面了
- mediainfo 现在显示路径了
- 得益于重写组件 解决了一些已知问题

- Added "Additional Content" line
- Added a "Recommended" row and "Same Artist Works" row for music page
- Added a loading spinner and no-image prompt for list item images
- Added network request tracing
- Added searching for settings in the settings window
- Changed the stack animation of the homepage to OVER_LEFT_RIGHT
- Decreased the size of the "Watched" tag
- Empty lists will no longer be displayed
- In the project page, tags/studios/genres can now be entered into the corresponding pages
- Integrated movie and series pages
- Made the first line of list items bold and decreased the list item font size
- Now able to display single tracks without an album
- Revised error handling
- Revised network requests, with URLs now able to be automatically formatted, but still requiring the addition of a protocol header
- Revised some components
- Revised some methods
- Revised the loading progress bar
- The "Likes" page no longer uses caching
- The "include" line now displays below the episode list
- The server list on the homepage now syncs with the outside
- When entering the project page from the collection, the title will now use the project title
- mediainfo now displays the path
- Due to the component rewrite, several known issues have been resolved.

## 0.5.0-1

- 增加了 clapper 作为播放器

> [!NOTE]
> clapper 在 Windows 上尚未解决兼容性问题，因此该版本依然默认启用 mpv，如果你使用 Linux 则可以尝试在设置中关闭 mpv 选项，启用 clapper。

- 重构了设置页，封装到 adwdialog
- 统一了列表标题的样式
- 右键菜单现在出现在点击事件的右下

- 修复了若干已知错误

- Added clapper as videso player

> [!NOTE]
> clapper has not yet resolved compatibility issues on Windows in this version, so it still defaults to using mpv. If you are using Linux, you can try disabling the mpv option in the settings and enabling clapper for better integration.

- Refactored the settings page, encapsulated into adwdialog
- Unified the style of list titles
- Right-click menus now appear at the bottom right of click events

- Fixed several known issues

## 0.5.0-preview

- 增加了 clapper 作为默认播放器

> [!NOTE]
> clapper 在 Windows 上尚未解决兼容性问题，因此该预览版依然默认启用 mpv，如果你使用 Linux 则可以尝试在设置中关闭 mpv 选项，启用 clapper。

- 重构了设置页，封装到 adwdialog
- 统一了列表标题的样式
- 右键菜单现在出现在点击事件的右下

- 修复了若干已知错误

- Added clapper as the default player

> [!NOTE]
> clapper has not yet resolved compatibility issues on Windows in this preview version, so it still defaults to using mpv. If you are using Linux, you can try disabling the mpv option in the settings and enabling clapper for better integration.

- Refactored the settings page, encapsulated into adwdialog
- Unified the style of list titles
- Right-click menus now appear at the bottom right of click events

- Fixed several known issues

## 0.4.13-1

- 增加首页每日推荐开关
- 增加字体显示设置

> [!NOTE]
> 如果你使用的字体包含的字符集不完整，将可能会出现缺字等显示异常

- Add a switch for daily recommendations on the homepage
- Add font display settings

> [!NOTE]
> If the font you are using has an incomplete character set, there may be missing characters or other display abnormalities.

## 0.4.13

### Added

- Added an auto-scrolling carousel
- Added window breakpoints
- Added initial music playback
- Added rating function
- Added hover tips

### Changed

- Moved history records to the homepage
- Changed GtkSpinner to GtkProgressBar
- Changed the mask style of the project details page
- Changed the style of some icons
- Removed window size limitations

### Fixed

- Fixed the issue where the distribution of episodes was blocked when entering the page from the season
- Fixed issues related to the cache directory
- Fixed several issues that may cause crashes
- Fixed the issue of being stuck in the mpv window when there are no available streams

## 0.4.10

### Added

- Added context menu
- Added option to remove item from resume list
- Added series page
- Added view collection for movie/series page
- Added reset password functionality
- Added search category
- Added album page (not playable for now)

### Changed

- List item second line has dim-label style enabled by default and scaled to 0.9
- Home page has title style enabled
- Rewrote bottom component for increased stability

### Fixed

- Fixed issue where password was required to be filled
- Fixed issue where spinner behaved incorrectly when logging in

## 0.4.9-1

### Hotfix

- Fixed backdrop view not display correctly

## 0.4.9

### Added

- Added quick logout button
- Added library sort: view and category
- Added font size option
- Added global local cache
- Added tags page
- Added transition animation when adding new item in the list
- Added empty library page hint

### Fixed

- Fixed image resizing when entering episode/movie details
- Fixed invisible items in some libraries
- Fixed adding account will login to the preference server directly

### Changed

- Deprecated secondary home update

## 0.4.5

### Breaking Changes (Windows only)

- ⚠️ **Breaking Change:** ⚠️ remove tsukimi.toml and relogin if you encounter login issues.

### Highlights

- Added multi accounts support

## 0.4.3

### Added

- Added search recommendation

### Changed

- Refactored search page
- Refactored play page

### Fixed

- Improved animation fluency

## 0.4.2

### Added

- feat: background blur **⚠️EXPERIMENTAL⚠️ GPU intensive**

### Changed

- chore: organize preferences

## 0.4.1

### Added

- feat: Actor page
- feat: Background
- feat: Search episodes in seasons
- feat: Recommend list
