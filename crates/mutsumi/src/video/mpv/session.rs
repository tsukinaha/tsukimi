use std::{
    fmt,
    time::Duration,
};

use super::proxy::BufferTransport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererCandidate {
    VulkanDmabuf,
    OpenGlDmabuf,
    WlShm,
}

impl RendererCandidate {
    pub(crate) fn transport(self) -> BufferTransport {
        match self {
            Self::VulkanDmabuf | Self::OpenGlDmabuf => BufferTransport::Dmabuf,
            Self::WlShm => BufferTransport::Shm,
        }
    }
}

impl fmt::Display for RendererCandidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::VulkanDmabuf => "Vulkan + linux-dmabuf",
            Self::OpenGlDmabuf => "OpenGL/EGL + linux-dmabuf",
            Self::WlShm => "wlshm",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FallbackPolicy {
    #[default]
    Auto,
    DmabufOnly,
    ShmOnly,
    Ordered(Vec<RendererCandidate>),
}

impl FallbackPolicy {
    pub(crate) fn candidates(&self) -> Vec<RendererCandidate> {
        let candidates = match self {
            Self::Auto => vec![
                RendererCandidate::VulkanDmabuf,
                RendererCandidate::OpenGlDmabuf,
                RendererCandidate::WlShm,
            ],
            Self::DmabufOnly => vec![
                RendererCandidate::VulkanDmabuf,
                RendererCandidate::OpenGlDmabuf,
            ],
            Self::ShmOnly => vec![RendererCandidate::WlShm],
            Self::Ordered(candidates) => candidates.clone(),
        };

        let mut unique = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if !unique.contains(&candidate) {
                unique.push(candidate);
            }
        }
        unique
    }
}

#[derive(Debug, Clone)]
pub struct MpvSessionOptions {
    pub fallback_policy: FallbackPolicy,
    pub first_frame_timeout: Duration,
}

impl Default for MpvSessionOptions {
    fn default() -> Self {
        Self {
            fallback_policy: FallbackPolicy::Auto,
            first_frame_timeout: Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CandidateFailure {
    #[error("failed to initialize MPV candidate: {0}")]
    Initialization(String),
    #[error("failed to start the synthetic Wayland compositor: {0}")]
    Proxy(String),
    #[error("MPV could not initialize the candidate video output")]
    VideoOutputInitialization,
    #[error("candidate produced {actual:?} buffers but requires {expected:?}")]
    UnexpectedTransport {
        expected: BufferTransport,
        actual: BufferTransport,
    },
    #[error(
        "DMA-BUF import failed for fourcc {fourcc:#010x}, modifier {modifier:#018x}: {message}"
    )]
    DmabufImport {
        fourcc: u32,
        modifier: u64,
        message: String,
    },
    #[error("SHM texture import failed: {0}")]
    ShmImport(String),
    #[error("the candidate did not produce an importable first frame before the watchdog expired")]
    FirstFrameTimeout,
    #[error("a user renderer override prevents automatic fallback: {0}")]
    UserOverride(String),
    #[error("the MPV session is unavailable: {0}")]
    Unavailable(String),
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpvSessionEvent {
    CandidateStarted {
        generation: u64,
        candidate: RendererCandidate,
    },
    CandidateFailed {
        generation: u64,
        candidate: RendererCandidate,
        reason: CandidateFailure,
        will_retry: bool,
    },
    Ready {
        generation: u64,
        candidate: RendererCandidate,
    },
    Failed {
        reason: CandidateFailure,
    },
}
