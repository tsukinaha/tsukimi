use crate::mpv_node::sys_node::SysMpvNode;
use crate::{mpv::mpv_err, *};

use std::ffi::{c_void, CString};
use std::os::raw as ctype;
use std::ptr::NonNull;
use std::slice;

/// An `Event`'s ID.
pub use libmpv2_sys::mpv_event_id as EventId;

use self::mpv_node::MpvNode;
pub mod mpv_event_id {
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_AUDIO_RECONFIG as AudioReconfig;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_CLIENT_MESSAGE as ClientMessage;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_COMMAND_REPLY as CommandReply;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_END_FILE as EndFile;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_FILE_LOADED as FileLoaded;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_GET_PROPERTY_REPLY as GetPropertyReply;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_HOOK as Hook;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_LOG_MESSAGE as LogMessage;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_NONE as None;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_PLAYBACK_RESTART as PlaybackRestart;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE as PropertyChange;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_QUEUE_OVERFLOW as QueueOverflow;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_SEEK as Seek;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_SET_PROPERTY_REPLY as SetPropertyReply;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_SHUTDOWN as Shutdown;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_START_FILE as StartFile;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_TICK as Tick;
    pub use libmpv2_sys::mpv_event_id_MPV_EVENT_VIDEO_RECONFIG as VideoReconfig;
}

#[derive(Debug)]
/// Data that is returned by both `GetPropertyReply` and `PropertyChange` events.
pub enum PropertyData<'a> {
    Str(&'a str),
    OsdStr(&'a str),
    Flag(bool),
    Int64(i64),
    Double(ctype::c_double),
    Node(MpvNode),
}

impl<'a> PropertyData<'a> {
    // SAFETY: meant to extract the data from an event property. See `mpv_event_property` in
    // `client.h`
    unsafe fn from_raw(format: MpvFormat, ptr: *mut ctype::c_void) -> Result<PropertyData<'a>> {
        assert!(!ptr.is_null());
        match format {
            mpv_format::Flag => Ok(PropertyData::Flag(*(ptr as *mut bool))),
            mpv_format::String => {
                let char_ptr = *(ptr as *mut *mut ctype::c_char);
                Ok(PropertyData::Str(mpv_cstr_to_str!(char_ptr)?))
            }
            mpv_format::OsdString => {
                let char_ptr = *(ptr as *mut *mut ctype::c_char);
                Ok(PropertyData::OsdStr(mpv_cstr_to_str!(char_ptr)?))
            }
            mpv_format::Double => Ok(PropertyData::Double(*(ptr as *mut f64))),
            mpv_format::Int64 => Ok(PropertyData::Int64(*(ptr as *mut i64))),
            mpv_format::Node => {
                let sys_node = *(ptr as *mut libmpv2_sys::mpv_node);
                let node = SysMpvNode::new(sys_node, false);
                return Ok(PropertyData::Node(node.value().unwrap()));
            }
            mpv_format::None => unreachable!(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum Event<'a> {
    /// Received when the player is shutting down
    Shutdown,
    /// *Has not been tested*, received when explicitly asked to MPV
    LogMessage {
        prefix: &'a str,
        level: &'a str,
        text: &'a str,
        log_level: LogLevel,
    },
    /// Received when using get_property_async
    GetPropertyReply {
        name: &'a str,
        result: PropertyData<'a>,
        reply_userdata: u64,
    },
    /// Received when using set_property_async
    SetPropertyReply(u64),
    /// Received when using command_async
    CommandReply(u64),
    /// Event received when a new file is playing
    StartFile,
    /// Event received when the file being played currently has stopped, for an error or not
    EndFile(EndFileReason),
    /// Event received when a file has been *loaded*, but has not been started
    FileLoaded,
    ClientMessage(Vec<&'a str>),
    VideoReconfig,
    AudioReconfig,
    /// The player changed current position
    Seek,
    PlaybackRestart,
    /// Received when used with observe_property
    PropertyChange {
        name: &'a str,
        change: PropertyData<'a>,
        reply_userdata: u64,
    },
    /// Received when the Event Queue is full
    QueueOverflow,
    /// A deprecated event
    Deprecated(libmpv2_sys::mpv_event),
}

unsafe extern "C" fn wu_wrapper<F: Fn() + Send + 'static>(ctx: *mut c_void) {
    if ctx.is_null() {
        panic!("ctx for wakeup wrapper is NULL");
    }

    (*(ctx as *mut F))();
}

/// Context to listen to events.
pub struct EventContext {
    ctx: NonNull<libmpv2_sys::mpv_handle>,
    wakeup_callback_cleanup: Option<Box<dyn FnOnce()>>,
}

unsafe impl Send for EventContext {}

impl EventContext {
    pub fn new(ctx: NonNull<libmpv2_sys::mpv_handle>) -> Self {
        EventContext {
            ctx,
            wakeup_callback_cleanup: None,
        }
    }

    /// Enable an event.
    pub fn enable_event(&self, ev: events::EventId) -> Result<()> {
        mpv_err((), unsafe {
            libmpv2_sys::mpv_request_event(self.ctx.as_ptr(), ev, 1)
        })
    }

    /// Enable all, except deprecated, events.
    pub fn enable_all_events(&self) -> Result<()> {
        for i in (2..9).chain(16..19).chain(20..23).chain(24..26) {
            self.enable_event(i)?;
        }
        Ok(())
    }

    /// Disable an event.
    pub fn disable_event(&self, ev: events::EventId) -> Result<()> {
        mpv_err((), unsafe {
            libmpv2_sys::mpv_request_event(self.ctx.as_ptr(), ev, 0)
        })
    }

    /// Diable all deprecated events.
    pub fn disable_deprecated_events(&self) -> Result<()> {
        self.disable_event(libmpv2_sys::mpv_event_id_MPV_EVENT_IDLE)?;
        Ok(())
    }

    /// Diable all events.
    pub fn disable_all_events(&self) -> Result<()> {
        for i in 2..26 {
            self.disable_event(i as _)?;
        }
        Ok(())
    }

    /// Observe `name` property for changes. `id` can be used to unobserve this (or many) properties
    /// again.
    pub fn observe_property(&self, name: &str, format: Format, id: u64) -> Result<()> {
        let name = CString::new(name)?;
        mpv_err((), unsafe {
            libmpv2_sys::mpv_observe_property(
                self.ctx.as_ptr(),
                id,
                name.as_ptr(),
                format.as_mpv_format() as _,
            )
        })
    }

    /// Unobserve any property associated with `id`.
    pub fn unobserve_property(&self, id: u64) -> Result<()> {
        mpv_err((), unsafe {
            libmpv2_sys::mpv_unobserve_property(self.ctx.as_ptr(), id)
        })
    }

    /// Wait for `timeout` seconds for an `Event`. Passing `0` as `timeout` will poll.
    /// For more information, as always, see the mpv-sys docs of `mpv_wait_event`.
    ///
    /// This function is intended to be called repeatedly in a wait-event loop.
    ///
    /// Returns `Some(Err(...))` if there was invalid utf-8, or if either an
    /// `MPV_EVENT_GET_PROPERTY_REPLY`, `MPV_EVENT_SET_PROPERTY_REPLY`, `MPV_EVENT_COMMAND_REPLY`,
    /// or `MPV_EVENT_PROPERTY_CHANGE` event failed, or if `MPV_EVENT_END_FILE` reported an error.
    pub fn wait_event(&mut self, timeout: f64) -> Option<Result<Event>> {
        let event = unsafe { *libmpv2_sys::mpv_wait_event(self.ctx.as_ptr(), timeout) };
        if event.event_id != mpv_event_id::None {
            if let Err(e) = mpv_err((), event.error) {
                return Some(Err(e));
            }
        }

        match event.event_id {
            mpv_event_id::None => None,
            mpv_event_id::Shutdown => Some(Ok(Event::Shutdown)),
            mpv_event_id::LogMessage => {
                let log_message =
                    unsafe { *(event.data as *mut libmpv2_sys::mpv_event_log_message) };

                let prefix = unsafe { mpv_cstr_to_str!(log_message.prefix) };
                Some(prefix.and_then(|prefix| {
                    Ok(Event::LogMessage {
                        prefix,
                        level: unsafe { mpv_cstr_to_str!(log_message.level)? },
                        text: unsafe { mpv_cstr_to_str!(log_message.text)? },
                        log_level: log_message.log_level,
                    })
                }))
            }
            mpv_event_id::GetPropertyReply => {
                let property = unsafe { *(event.data as *mut libmpv2_sys::mpv_event_property) };

                let name = unsafe { mpv_cstr_to_str!(property.name) };
                Some(name.and_then(|name| {
                    // SAFETY: safe because we are passing format + data from an mpv_event_property
                    let result = unsafe { PropertyData::from_raw(property.format, property.data) }?;

                    Ok(Event::GetPropertyReply {
                        name,
                        result,
                        reply_userdata: event.reply_userdata,
                    })
                }))
            }
            mpv_event_id::SetPropertyReply => Some(mpv_err(
                Event::SetPropertyReply(event.reply_userdata),
                event.error,
            )),
            mpv_event_id::CommandReply => Some(mpv_err(
                Event::CommandReply(event.reply_userdata),
                event.error,
            )),
            mpv_event_id::StartFile => Some(Ok(Event::StartFile)),
            mpv_event_id::EndFile => {
                let end_file = unsafe { *(event.data as *mut libmpv2_sys::mpv_event_end_file) };

                if let Err(e) = mpv_err((), end_file.error) {
                    Some(Err(e))
                } else {
                    Some(Ok(Event::EndFile(end_file.reason as _)))
                }
            }
            mpv_event_id::FileLoaded => Some(Ok(Event::FileLoaded)),
            mpv_event_id::ClientMessage => {
                let client_message =
                    unsafe { *(event.data as *mut libmpv2_sys::mpv_event_client_message) };
                let messages = unsafe {
                    slice::from_raw_parts_mut(client_message.args, client_message.num_args as _)
                };
                Some(Ok(Event::ClientMessage(
                    messages
                        .iter()
                        .map(|msg| unsafe { mpv_cstr_to_str!(*msg) })
                        .collect::<Result<Vec<_>>>()
                        .unwrap(),
                )))
            }
            mpv_event_id::VideoReconfig => Some(Ok(Event::VideoReconfig)),
            mpv_event_id::AudioReconfig => Some(Ok(Event::AudioReconfig)),
            mpv_event_id::Seek => Some(Ok(Event::Seek)),
            mpv_event_id::PlaybackRestart => Some(Ok(Event::PlaybackRestart)),
            mpv_event_id::PropertyChange => {
                let property = unsafe { *(event.data as *mut libmpv2_sys::mpv_event_property) };

                // This happens if the property is not available. For example,
                // if you reached EndFile while observing a property.
                if property.format == mpv_format::None {
                    None
                } else {
                    let name = unsafe { mpv_cstr_to_str!(property.name) };
                    Some(name.and_then(|name| {
                        // SAFETY: safe because we are passing format + data from an mpv_event_property
                        let change =
                            unsafe { PropertyData::from_raw(property.format, property.data) }?;

                        Ok(Event::PropertyChange {
                            name,
                            change,
                            reply_userdata: event.reply_userdata,
                        })
                    }))
                }
            }
            mpv_event_id::QueueOverflow => Some(Ok(Event::QueueOverflow)),
            _ => Some(Ok(Event::Deprecated(event))),
        }
    }

    /// Set a custom function that should be called when there are new events. Use this if
    /// blocking in [wait_event](#method.wait_event) to wait for new events is not feasible.
    ///
    /// Keep in mind that the callback will be called from foreign threads. You must not make
    /// any assumptions of the environment, and you must return as soon as possible (i.e. no
    /// long blocking waits). Exiting the callback through any other means than a normal return
    /// is forbidden (no throwing exceptions, no `longjmp()` calls). You must not change any
    /// local thread state (such as the C floating point environment).
    ///
    /// You are not allowed to call any client API functions inside of the callback. In
    /// particular, you should not do any processing in the callback, but wake up another
    /// thread that does all the work. The callback is meant strictly for notification only,
    /// and is called from arbitrary core parts of the player, that make no considerations for
    /// reentrant API use or allowing the callee to spend a lot of time doing other things.
    /// Keep in mind that it’s also possible that the callback is called from a thread while a
    /// mpv API function is called (i.e. it can be reentrant).
    ///
    /// In general, the client API expects you to call [wait_event](#method.wait_event) to receive
    /// notifications, and the wakeup callback is merely a helper utility to make this easier in
    /// certain situations. Note that it’s possible that there’s only one wakeup callback
    /// invocation for multiple events. You should call [wait_event](#method.wait_event) with no timeout until
    /// `None` is returned, at which point the event queue is empty.
    ///
    /// If you actually want to do processing in a callback, spawn a thread that does nothing but
    /// call [wait_event](#method.wait_event) in a loop and dispatches the result to a callback.
    ///
    /// Only one wakeup callback can be set.
    pub fn set_wakeup_callback<F: Fn() + Send + 'static>(&mut self, callback: F) {
        if let Some(wakeup_callback_cleanup) = self.wakeup_callback_cleanup.take() {
            wakeup_callback_cleanup();
        }
        let raw_callback = Box::into_raw(Box::new(callback));
        self.wakeup_callback_cleanup = Some(Box::new(move || unsafe {
            drop(Box::from_raw(raw_callback));
        }) as Box<dyn FnOnce()>);
        unsafe {
            libmpv2_sys::mpv_set_wakeup_callback(
                self.ctx.as_ptr(),
                Some(wu_wrapper::<F>),
                raw_callback as *mut c_void,
            );
        }
    }
}

impl Drop for EventContext {
    fn drop(&mut self) {
        if let Some(wakeup_callback_cleanup) = self.wakeup_callback_cleanup.take() {
            wakeup_callback_cleanup();
        }
    }
}
