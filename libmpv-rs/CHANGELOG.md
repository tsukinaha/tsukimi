# Changelog

## Unreleased

## Version 2.0.1
* Fix `playlist_previous_*` commands using wrong mpv command ([issue](https://github.com/ParadoxSpiral/libmpv-rs/issues/17))
* Use local libmpv-sys as dependency except on crates.io

## Version 2.0.0
* Add method `Mpv::with_initializer` to set options before initialization
* [breaking] Borrow `&mut self` in `wait_event` to disallow using two events where the first points to data freed in the second `wait_event` call
* [breaking] `PropertyData<'_>` is no longer `Clone` or `PartialEq`, `Event<'_>` is no longer `Clone` to avoid cloning/comparing `MpvNode`

## Version 1.1.0
* Add an `MpvNode` that implements `GetData`, i.a. with `MpvNodeArrayIter` and `MpvNodeMapIter` variants that support e.g. properties `audio-parmas` and `playlist`

## Version 1.0.1
* Use debug formatting in impl of `Display` trait for `Error`
