// Copyright (C) 2016  ParadoxSpiral
//
// This file is part of mpv-rs.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA

use super::*;

use std::alloc::{self, Layout};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::os::raw as ctype;
use std::panic;
use std::panic::RefUnwindSafe;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::{atomic::Ordering, Mutex};

impl Mpv {
    /// Create a context with which custom protocols can be registered.
    ///
    /// # Panics
    /// Panics if a context already exists
    pub fn create_protocol_context<T, U>(&self) -> ProtocolContext<T, U>
    where
        T: RefUnwindSafe,
        U: RefUnwindSafe,
    {
        match self.protocols_guard.compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => ProtocolContext::new(self.ctx, PhantomData::<&Self>),
            Err(_) => panic!("A protocol context already exists"),
        }
    }
}

/// Return a persistent `T` that is passed to all other `Stream*` functions, panic on errors.
pub type StreamOpen<T, U> = fn(&mut U, &str) -> T;
/// Do any necessary cleanup.
pub type StreamClose<T> = fn(Box<T>);
/// Seek to the given offset. Return the new offset, or either `MpvError::Generic` if seeking
/// failed or panic.
pub type StreamSeek<T> = fn(&mut T, i64) -> i64;
/// Target buffer with fixed capacity.
/// Return either the number of read bytes, `0` on EOF, or either `-1` or panic on error.
pub type StreamRead<T> = fn(&mut T, &mut [ctype::c_char]) -> i64;
/// Return the total size of the stream in bytes. Panic on error.
pub type StreamSize<T> = fn(&mut T) -> i64;

unsafe extern "C" fn open_wrapper<T, U>(
    user_data: *mut ctype::c_void,
    uri: *mut ctype::c_char,
    info: *mut libmpv_sys::mpv_stream_cb_info,
) -> ctype::c_int
where
    T: RefUnwindSafe,
    U: RefUnwindSafe,
{
    let data = user_data as *mut ProtocolData<T, U>;

    (*info).cookie = user_data;
    (*info).read_fn = Some(read_wrapper::<T, U>);
    (*info).seek_fn = Some(seek_wrapper::<T, U>);
    (*info).size_fn = Some(size_wrapper::<T, U>);
    (*info).close_fn = Some(close_wrapper::<T, U>);

    let ret = panic::catch_unwind(|| {
        let uri = mpv_cstr_to_str!(uri as *const _).unwrap();
        ptr::write(
            (*data).cookie,
            ((*data).open_fn)(&mut (*data).user_data, uri),
        );
    });

    if ret.is_ok() {
        0
    } else {
        mpv_error::Generic as _
    }
}

unsafe extern "C" fn read_wrapper<T, U>(
    cookie: *mut ctype::c_void,
    buf: *mut ctype::c_char,
    nbytes: u64,
) -> i64
where
    T: RefUnwindSafe,
    U: RefUnwindSafe,
{
    let data = cookie as *mut ProtocolData<T, U>;

    let ret = panic::catch_unwind(|| {
        let slice = slice::from_raw_parts_mut(buf, nbytes as _);
        ((*data).read_fn)(&mut *(*data).cookie, slice)
    });
    if let Ok(ret) = ret {
        ret
    } else {
        -1
    }
}

unsafe extern "C" fn seek_wrapper<T, U>(cookie: *mut ctype::c_void, offset: i64) -> i64
where
    T: RefUnwindSafe,
    U: RefUnwindSafe,
{
    let data = cookie as *mut ProtocolData<T, U>;

    if (*data).seek_fn.is_none() {
        return mpv_error::Unsupported as _;
    }

    let ret =
        panic::catch_unwind(|| (*(*data).seek_fn.as_ref().unwrap())(&mut *(*data).cookie, offset));
    if let Ok(ret) = ret {
        ret
    } else {
        mpv_error::Generic as _
    }
}

unsafe extern "C" fn size_wrapper<T, U>(cookie: *mut ctype::c_void) -> i64
where
    T: RefUnwindSafe,
    U: RefUnwindSafe,
{
    let data = cookie as *mut ProtocolData<T, U>;

    if (*data).size_fn.is_none() {
        return mpv_error::Unsupported as _;
    }

    let ret = panic::catch_unwind(|| (*(*data).size_fn.as_ref().unwrap())(&mut *(*data).cookie));
    if let Ok(ret) = ret {
        ret
    } else {
        mpv_error::Unsupported as _
    }
}

#[allow(unused_must_use)]
unsafe extern "C" fn close_wrapper<T, U>(cookie: *mut ctype::c_void)
where
    T: RefUnwindSafe,
    U: RefUnwindSafe,
{
    let data = Box::from_raw(cookie as *mut ProtocolData<T, U>);

    panic::catch_unwind(|| ((*data).close_fn)(Box::from_raw((*data).cookie)));
}

struct ProtocolData<T, U> {
    cookie: *mut T,
    user_data: U,

    open_fn: StreamOpen<T, U>,
    close_fn: StreamClose<T>,
    read_fn: StreamRead<T>,
    seek_fn: Option<StreamSeek<T>>,
    size_fn: Option<StreamSize<T>>,
}

/// This context holds state relevant to custom protocols.
/// It is created by calling `Mpv::create_protocol_context`.
pub struct ProtocolContext<'parent, T: RefUnwindSafe, U: RefUnwindSafe> {
    ctx: NonNull<libmpv_sys::mpv_handle>,
    protocols: Mutex<Vec<Protocol<T, U>>>,
    _does_not_outlive: PhantomData<&'parent Mpv>,
}

unsafe impl<'parent, T: RefUnwindSafe, U: RefUnwindSafe> Send for ProtocolContext<'parent, T, U> {}
unsafe impl<'parent, T: RefUnwindSafe, U: RefUnwindSafe> Sync for ProtocolContext<'parent, T, U> {}

impl<'parent, T: RefUnwindSafe, U: RefUnwindSafe> ProtocolContext<'parent, T, U> {
    fn new(
        ctx: NonNull<libmpv_sys::mpv_handle>,
        marker: PhantomData<&'parent Mpv>,
    ) -> ProtocolContext<'parent, T, U> {
        ProtocolContext {
            ctx,
            protocols: Mutex::new(Vec::new()),
            _does_not_outlive: marker,
        }
    }

    /// Register a custom `Protocol`. Once a protocol has been registered, it lives as long as
    /// `Mpv`.
    ///
    /// Returns `Error::Mpv(MpvError::InvalidParameter)` if a protocol with the same name has
    /// already been registered.
    pub fn register(&self, protocol: Protocol<T, U>) -> Result<()> {
        let mut protocols = self.protocols.lock().unwrap();
        protocol.register(self.ctx.as_ptr())?;
        protocols.push(protocol);
        Ok(())
    }
}

/// `Protocol` holds all state used by a custom protocol.
pub struct Protocol<T: Sized + RefUnwindSafe, U: RefUnwindSafe> {
    name: String,
    data: *mut ProtocolData<T, U>,
}

impl<T: RefUnwindSafe, U: RefUnwindSafe> Protocol<T, U> {
    /// `name` is the prefix of the protocol, e.g. `name://path`.
    ///
    /// `user_data` is data that will be passed to `open_fn`.
    ///
    /// # Safety
    /// Do not call libmpv functions in any supplied function.
    /// All panics of the provided functions are catched and can be used as generic error returns.
    pub unsafe fn new(
        name: String,
        user_data: U,
        open_fn: StreamOpen<T, U>,
        close_fn: StreamClose<T>,
        read_fn: StreamRead<T>,
        seek_fn: Option<StreamSeek<T>>,
        size_fn: Option<StreamSize<T>>,
    ) -> Protocol<T, U> {
        let c_layout = Layout::from_size_align(mem::size_of::<T>(), mem::align_of::<T>()).unwrap();
        let cookie = alloc::alloc(c_layout) as *mut T;
        let data = Box::into_raw(Box::new(ProtocolData {
            cookie,
            user_data,

            open_fn,
            close_fn,
            read_fn,
            seek_fn,
            size_fn,
        }));

        Protocol { name, data }
    }

    fn register(&self, ctx: *mut libmpv_sys::mpv_handle) -> Result<()> {
        let name = CString::new(&self.name[..])?;
        unsafe {
            mpv_err(
                (),
                libmpv_sys::mpv_stream_cb_add_ro(
                    ctx,
                    name.as_ptr(),
                    self.data as *mut _,
                    Some(open_wrapper::<T, U>),
                ),
            )
        }
    }
}
