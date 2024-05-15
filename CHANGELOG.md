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
