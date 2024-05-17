#![cfg_attr(docsrs, feature(doc_cfg))]

macro_rules! assert_initialized_main_thread {
    () => {};
}

macro_rules! skip_assert_initialized {
    () => {};
}

#[allow(unused_imports)]
mod auto;
pub use crate::auto::*;
