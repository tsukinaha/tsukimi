macro_rules! mpv_cstr_to_str {
    ($cstr: expr) => {
        std::ffi::CStr::from_ptr($cstr)
            .to_str()
            .map_err(Error::from)
    };
}

mod errors;

/// Event handling
pub mod events;
/// Custom protocols (`protocol://$url`) for playback
#[cfg(feature = "protocols")]
pub mod protocol;
/// Custom rendering
#[cfg(feature = "render")]
pub mod render;

pub use self::errors::*;
use self::events::EventContext;
use super::*;

use std::{
    ffi::CString,
    mem::MaybeUninit,
    ops::Deref,
    ptr::{self, NonNull},
    sync::atomic::AtomicBool,
};

fn mpv_err<T>(ret: T, err: ctype::c_int) -> Result<T> {
    if err == 0 {
        Ok(ret)
    } else {
        Err(Error::Raw(err))
    }
}

/// This trait describes which types are allowed to be passed to getter mpv APIs.
pub unsafe trait GetData: Sized {
    #[doc(hidden)]
    fn get_from_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(mut fun: F) -> Result<Self> {
        let mut val = MaybeUninit::uninit();
        let _ = fun(val.as_mut_ptr() as *mut _)?;
        Ok(unsafe { val.assume_init() })
    }
    fn get_format() -> Format;
}

/// This trait describes which types are allowed to be passed to setter mpv APIs.
pub unsafe trait SetData: Sized {
    #[doc(hidden)]
    fn call_as_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(
        mut self,
        mut fun: F,
    ) -> Result<T> {
        fun(&mut self as *mut Self as _)
    }
    fn get_format() -> Format;
}

unsafe impl GetData for f64 {
    fn get_format() -> Format {
        Format::Double
    }
}

unsafe impl SetData for f64 {
    fn get_format() -> Format {
        Format::Double
    }
}

unsafe impl GetData for i64 {
    fn get_format() -> Format {
        Format::Int64
    }
}

pub mod mpv_node {
    use self::sys_node::SysMpvNode;
    use crate::{Error, Format, GetData, Result};
    use std::{mem::MaybeUninit, os::raw::c_void, ptr};

    #[derive(Debug, Clone)]
    pub enum MpvNode {
        String(String),
        Flag(bool),
        Int64(i64),
        Double(f64),
        ArrayIter(MpvNodeArrayIter),
        MapIter(MpvNodeMapIter),
        None,
    }

    impl MpvNode {
        pub fn bool(&self) -> Option<bool> {
            if let MpvNode::Flag(value) = *self {
                Some(value)
            } else {
                None
            }
        }
        pub fn i64(&self) -> Option<i64> {
            if let MpvNode::Int64(value) = *self {
                Some(value)
            } else {
                None
            }
        }
        pub fn f64(&self) -> Option<f64> {
            if let MpvNode::Double(value) = *self {
                Some(value)
            } else {
                None
            }
        }

        pub fn str(&self) -> Option<&str> {
            if let MpvNode::String(value) = self {
                Some(value)
            } else {
                None
            }
        }

        pub fn array(self) -> Option<MpvNodeArrayIter> {
            if let MpvNode::ArrayIter(value) = self {
                Some(value)
            } else {
                None
            }
        }

        pub fn map(self) -> Option<MpvNodeMapIter> {
            if let MpvNode::MapIter(value) = self {
                Some(value)
            } else {
                None
            }
        }
    }

    impl PartialEq for MpvNode {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::String(l0), Self::String(r0)) => l0 == r0,
                (Self::Flag(l0), Self::Flag(r0)) => l0 == r0,
                (Self::Int64(l0), Self::Int64(r0)) => l0 == r0,
                (Self::Double(l0), Self::Double(r0)) => l0 == r0,
                (Self::ArrayIter(l0), Self::ArrayIter(r0)) => l0.clone().eq(r0.clone()),
                (Self::MapIter(l0), Self::MapIter(r0)) => l0.clone().eq(r0.clone()),
                _ => core::mem::discriminant(self) == core::mem::discriminant(other),
            }
        }
    }

    #[derive(Debug)]
    struct DropWrapper(libmpv2_sys::mpv_node);

    impl Drop for DropWrapper {
        fn drop(&mut self) {
            unsafe {
                libmpv2_sys::mpv_free_node_contents(&mut self.0 as *mut libmpv2_sys::mpv_node)
            };
        }
    }

    pub mod sys_node {
        use super::{DropWrapper, MpvNode, MpvNodeArrayIter, MpvNodeMapIter};
        use crate::{mpv_error, mpv_format, Error, Result};
        use std::rc::Rc;

        #[derive(Debug, Clone)]
        pub struct SysMpvNode {
            // Reference counted pointer to a parent node so it stays alive long enough.
            //
            // MPV has one big cleanup function that takes a node so store the parent node
            // and force it to stay alive until the reference count hits 0.
            parent: Option<Rc<DropWrapper>>,
            node: libmpv2_sys::mpv_node,
        }

        impl SysMpvNode {
            pub fn new(node: libmpv2_sys::mpv_node, drop: bool) -> Self {
                Self {
                    parent: if drop {
                        Some(Rc::new(DropWrapper(node)))
                    } else {
                        None
                    },
                    node,
                }
            }

            pub fn child(self: Self, node: libmpv2_sys::mpv_node) -> Self {
                Self {
                    parent: self.parent,
                    node,
                }
            }

            pub fn value(&self) -> Result<MpvNode> {
                let node = self.node;
                Ok(match node.format {
                    mpv_format::Flag => MpvNode::Flag(unsafe { node.u.flag } == 1),
                    mpv_format::Int64 => MpvNode::Int64(unsafe { node.u.int64 }),
                    mpv_format::Double => MpvNode::Double(unsafe { node.u.double_ }),
                    mpv_format::String => {
                        let text = unsafe { mpv_cstr_to_str!(node.u.string) }?.to_owned();
                        MpvNode::String(text)
                    }
                    mpv_format::Array => {
                        let list = unsafe { *node.u.list };
                        let iter = MpvNodeArrayIter {
                            node: self.clone(),
                            start: unsafe { *node.u.list }.values,
                            end: unsafe { list.values.offset(list.num.try_into().unwrap()) },
                        };
                        return Ok(MpvNode::ArrayIter(iter));
                    }

                    mpv_format::Map => MpvNode::MapIter(MpvNodeMapIter {
                        list: unsafe { *node.u.list },
                        curr: 0,
                        node: self.clone(),
                    }),
                    mpv_format::None => MpvNode::None,
                    _ => return Err(Error::Raw(mpv_error::PropertyError)),
                })
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct MpvNodeArrayIter {
        // Reference counted pointer to a parent node so it stays alive long enough.
        //
        // MPV has one big cleanup function that takes a node so store the parent node
        // and force it to stay alive until the reference count hits 0.
        node: SysMpvNode,
        start: *const libmpv2_sys::mpv_node,
        end: *const libmpv2_sys::mpv_node,
    }

    impl Iterator for MpvNodeArrayIter {
        type Item = MpvNode;

        fn next(&mut self) -> Option<Self::Item> {
            if self.start == self.end {
                None
            } else {
                unsafe {
                    let result = ptr::read(self.start);
                    let node = SysMpvNode::child(self.node.clone(), result);
                    self.start = self.start.offset(1);
                    node.value().ok()
                }
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct MpvNodeMapIter {
        // Reference counted pointer to a parent node so it stays alive long enough.
        //
        // MPV has one big cleanup function that takes a node so store the parent node
        // and force it to stay alive until the reference count hits 0.
        node: SysMpvNode,
        list: libmpv2_sys::mpv_node_list,
        curr: usize,
    }

    impl Iterator for MpvNodeMapIter {
        type Item = (String, MpvNode);

        fn next(&mut self) -> Option<Self::Item> {
            if self.curr >= self.list.num.try_into().unwrap() {
                None
            } else {
                let offset = self.curr.try_into().unwrap();
                let (key, value) = unsafe {
                    (
                        mpv_cstr_to_str!(*self.list.keys.offset(offset)),
                        *self.list.values.offset(offset),
                    )
                };
                self.curr += 1;
                let node = SysMpvNode::child(self.node.clone(), value);
                Some((key.unwrap().to_string(), node.value().unwrap()))
            }
        }
    }

    unsafe impl GetData for MpvNode {
        fn get_from_c_void<T, F: FnMut(*mut c_void) -> Result<T>>(mut fun: F) -> Result<Self> {
            let mut val = MaybeUninit::uninit();
            fun(val.as_mut_ptr() as *mut _)?;
            let sys_node = unsafe { val.assume_init() };
            let node = SysMpvNode::new(sys_node, true);
            node.value()
        }

        fn get_format() -> Format {
            Format::Node
        }
    }
}

unsafe impl SetData for i64 {
    fn get_format() -> Format {
        Format::Int64
    }
}

unsafe impl GetData for bool {
    fn get_format() -> Format {
        Format::Flag
    }
}

unsafe impl SetData for bool {
    fn call_as_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(self, mut fun: F) -> Result<T> {
        let mut cpy: i64 = if self { 1 } else { 0 };
        fun(&mut cpy as *mut i64 as *mut _)
    }

    fn get_format() -> Format {
        Format::Flag
    }
}

unsafe impl GetData for String {
    fn get_from_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(mut fun: F) -> Result<String> {
        let ptr = &mut ptr::null();
        fun(ptr as *mut *const ctype::c_char as _)?;

        let ret = unsafe { mpv_cstr_to_str!(*ptr) }?.to_owned();
        unsafe { libmpv2_sys::mpv_free(*ptr as *mut _) };
        Ok(ret)
    }

    fn get_format() -> Format {
        Format::String
    }
}

unsafe impl SetData for String {
    fn call_as_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(self, mut fun: F) -> Result<T> {
        let string = CString::new(self)?;
        fun((&mut string.as_ptr()) as *mut *const ctype::c_char as *mut _)
    }

    fn get_format() -> Format {
        Format::String
    }
}

/// Wrapper around an `&str` returned by mpv, that properly deallocates it with mpv's allocator.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct MpvStr<'a>(&'a str);
impl<'a> Deref for MpvStr<'a> {
    type Target = str;

    fn deref(&self) -> &str {
        self.0
    }
}
impl<'a> Drop for MpvStr<'a> {
    fn drop(&mut self) {
        unsafe { libmpv2_sys::mpv_free(self.0.as_ptr() as *mut u8 as _) };
    }
}

unsafe impl<'a> GetData for MpvStr<'a> {
    fn get_from_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(
        mut fun: F,
    ) -> Result<MpvStr<'a>> {
        let ptr = &mut ptr::null();
        let _ = fun(ptr as *mut *const ctype::c_char as _)?;

        Ok(MpvStr(unsafe { mpv_cstr_to_str!(*ptr) }?))
    }

    fn get_format() -> Format {
        Format::String
    }
}

unsafe impl<'a> SetData for &'a str {
    fn call_as_c_void<T, F: FnMut(*mut ctype::c_void) -> Result<T>>(self, mut fun: F) -> Result<T> {
        let string = CString::new(self)?;
        fun((&mut string.as_ptr()) as *mut *const ctype::c_char as *mut _)
    }

    fn get_format() -> Format {
        Format::String
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// Subset of `mpv_format` used by the public API.
pub enum Format {
    String,
    Flag,
    Int64,
    Double,
    Node,
}

impl Format {
    fn as_mpv_format(&self) -> MpvFormat {
        match *self {
            Format::String => mpv_format::String,
            Format::Flag => mpv_format::Flag,
            Format::Int64 => mpv_format::Int64,
            Format::Double => mpv_format::Double,
            Format::Node => mpv_format::Node,
        }
    }
}

/// Context passed to the `initializer` of `Mpv::with_initialzer`.
pub struct MpvInitializer {
    ctx: *mut libmpv2_sys::mpv_handle,
}

impl MpvInitializer {
    /// Set the value of a property.
    pub fn set_property<T: SetData>(&self, name: &str, data: T) -> Result<()> {
        let name = CString::new(name)?;
        let format = T::get_format().as_mpv_format() as _;
        data.call_as_c_void(|ptr| {
            mpv_err((), unsafe {
                libmpv2_sys::mpv_set_property(self.ctx, name.as_ptr(), format, ptr)
            })
        })
    }

    /// Set the value of an option
    pub fn set_option<T: SetData>(&self, name: &str, data: T) -> Result<()> {
        let name = CString::new(name)?;
        let format = T::get_format().as_mpv_format() as _;
        data.call_as_c_void(|ptr| {
            mpv_err((), unsafe {
                libmpv2_sys::mpv_set_option(self.ctx, name.as_ptr(), format, ptr)
            })
        })
    }
}

/// The central mpv context.
pub struct Mpv {
    /// The handle to the mpv core
    pub ctx: NonNull<libmpv2_sys::mpv_handle>,
    event_context: EventContext,
    #[cfg(feature = "protocols")]
    protocols_guard: AtomicBool,
}

unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe {
            libmpv2_sys::mpv_terminate_destroy(self.ctx.as_ptr());
        }
    }
}

impl Mpv {
    /// Create a new `Mpv`.
    /// The default settings can be probed by running: `$ mpv --show-profile=libmpv`.
    pub fn new() -> Result<Mpv> {
        Mpv::with_initializer(|_| Ok(()))
    }

    /// Create a new `Mpv`.
    /// The same as `Mpv::new`, but you can set properties before `Mpv` is initialized.
    pub fn with_initializer<F: FnOnce(MpvInitializer) -> Result<()>>(
        initializer: F,
    ) -> Result<Mpv> {
        let api_version = unsafe { libmpv2_sys::mpv_client_api_version() };
        if crate::MPV_CLIENT_API_MAJOR != api_version >> 16 {
            return Err(Error::VersionMismatch {
                linked: crate::MPV_CLIENT_API_VERSION,
                loaded: api_version,
            });
        }

        let ctx = unsafe { libmpv2_sys::mpv_create() };
        if ctx.is_null() {
            return Err(Error::Null);
        }

        initializer(MpvInitializer { ctx })?;
        mpv_err((), unsafe { libmpv2_sys::mpv_initialize(ctx) }).map_err(|err| {
            unsafe { libmpv2_sys::mpv_terminate_destroy(ctx) };
            err
        })?;

        let ctx = unsafe { NonNull::new_unchecked(ctx) };

        Ok(Mpv {
            ctx,
            event_context: EventContext::new(ctx),
            #[cfg(feature = "protocols")]
            protocols_guard: AtomicBool::new(false),
        })
    }

    /// Load a configuration file. The path has to be absolute, and a file.
    pub fn load_config(&self, path: &str) -> Result<()> {
        let file = CString::new(path)?.into_raw();
        let ret = mpv_err((), unsafe {
            libmpv2_sys::mpv_load_config_file(self.ctx.as_ptr(), file)
        });
        unsafe {
            drop(CString::from_raw(file));
        };
        ret
    }

    pub fn event_context(&self) -> &EventContext {
        &self.event_context
    }

    pub fn event_context_mut(&mut self) -> &mut EventContext {
        &mut self.event_context
    }

    /// Send a command to the `Mpv` instance. This uses `mpv_command_string` internally,
    /// so that the syntax is the same as described in the [manual for the input.conf](https://mpv.io/manual/master/#list-of-input-commands).
    ///
    /// Note that you may have to escape strings with `""` when they contain spaces.
    ///
    /// # Examples
    ///
    /// ```
    /// # use libmpv2::{Mpv};
    /// # use libmpv2::mpv_node::MpvNode;
    /// # use std::collections::HashMap;
    /// mpv.command("loadfile", &["test-data/jellyfish.mp4", "append-play"]).unwrap();
    /// # let node = mpv.get_property::<MpvNode>("playlist").unwrap();
    /// # let mut list = node.array().unwrap().collect::<Vec<_>>();
    /// # let map = list.pop().unwrap().map().unwrap().collect::<HashMap<_, _>>();
    /// # assert_eq!(map, HashMap::from([(String::from("id"), MpvNode::Int64(1)), (String::from("current"), MpvNode::Flag(true)), (String::from("filename"), MpvNode::String(String::from("test-data/jellyfish.mp4")))]));
    /// ```
    pub fn command(&self, name: &str, args: &[&str]) -> Result<()> {
        let mut cmd = name.to_owned();

        for elem in args {
            cmd.push(' ');
            cmd.push_str(elem);
        }

        let raw = CString::new(cmd)?;
        mpv_err((), unsafe {
            libmpv2_sys::mpv_command_string(self.ctx.as_ptr(), raw.as_ptr())
        })
    }

    /// Set the value of a property.
    pub fn set_property<T: SetData>(&self, name: &str, data: T) -> Result<()> {
        let name = CString::new(name)?;
        let format = T::get_format().as_mpv_format() as _;
        data.call_as_c_void(|ptr| {
            mpv_err((), unsafe {
                libmpv2_sys::mpv_set_property(self.ctx.as_ptr(), name.as_ptr(), format, ptr)
            })
        })
    }

    /// Get the value of a property.
    pub fn get_property<T: GetData>(&self, name: &str) -> Result<T> {
        let name = CString::new(name)?;

        let format = T::get_format().as_mpv_format() as _;
        T::get_from_c_void(|ptr| {
            mpv_err((), unsafe {
                libmpv2_sys::mpv_get_property(self.ctx.as_ptr(), name.as_ptr(), format, ptr)
            })
        })
    }

    /// Internal time in microseconds, this has an arbitrary offset, and will never go backwards.
    ///
    /// This can be called at any time, even if it was stated that no API function should be called.
    pub fn get_internal_time(&self) -> i64 {
        unsafe { libmpv2_sys::mpv_get_time_us(self.ctx.as_ptr()) }
    }
}
