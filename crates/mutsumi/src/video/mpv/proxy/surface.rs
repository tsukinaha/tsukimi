use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
};

use wl_proxy::{
    object::ObjectCoreApi,
    protocols::wayland::{
        wl_buffer::WlBuffer,
        wl_callback::WlCallback,
        wl_compositor::{
            WlCompositor,
            WlCompositorHandler,
        },
        wl_region::{
            WlRegion,
            WlRegionHandler,
        },
        wl_subcompositor::{
            WlSubcompositor,
            WlSubcompositorHandler,
        },
        wl_subsurface::{
            WlSubsurface,
            WlSubsurfaceHandler,
        },
        wl_surface::{
            WlSurface,
            WlSurfaceHandler,
        },
    },
};

use super::{
    CapturedFrame,
    FrameCallbacks,
    SharedState,
    SurfaceContentUpdate,
    SurfaceUpdate,
    selection::{
        ContentAction,
        content_action,
    },
    shm::ShmSnapshot,
    surface_tree::BufferMetadata,
};

#[derive(Default)]
pub(super) struct SurfaceRuntime {
    pending_buffer: Option<Option<Rc<WlBuffer>>>,
    pending_callbacks: Vec<Rc<WlCallback>>,
    latched: Option<SurfaceCommit>,
    current: Option<CurrentContent>,
}

#[derive(Default)]
struct SurfaceCommit {
    buffer: Option<Option<AttachedBuffer>>,
    callbacks: Vec<Rc<WlCallback>>,
}

struct AttachedBuffer {
    buffer: Rc<WlBuffer>,
    retained: bool,
}

#[derive(Clone)]
struct CurrentContent {
    buffer_id: u64,
    metadata: BufferMetadata,
    retains_buffer: bool,
    payload: CurrentPayload,
}

#[derive(Clone)]
enum CurrentPayload {
    Dmabuf,
    Shm(ShmSnapshot),
}

pub struct CompositorHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WlCompositorHandler for CompositorHandler {
    fn handle_create_surface(&mut self, _slf: &Rc<WlCompositor>, id: &Rc<WlSurface>) {
        id.set_forward_to_server(false);
        id.set_handler(SurfaceHandler {
            shared: Rc::clone(&self.state),
        });
        self.state.borrow_mut().insert_surface(id);
    }

    fn handle_create_region(&mut self, _slf: &Rc<WlCompositor>, id: &Rc<WlRegion>) {
        id.set_forward_to_server(false);
        id.set_handler(RegionHandler);
    }

    fn handle_release(&mut self, slf: &Rc<WlCompositor>) {
        slf.delete_id();
    }
}

struct RegionHandler;

impl WlRegionHandler for RegionHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlRegion>) {
        slf.delete_id();
    }

    fn handle_add(&mut self, _slf: &Rc<WlRegion>, _x: i32, _y: i32, _width: i32, _height: i32) {}

    fn handle_subtract(
        &mut self, _slf: &Rc<WlRegion>, _x: i32, _y: i32, _width: i32, _height: i32,
    ) {
    }
}

pub struct SubcompositorHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WlSubcompositorHandler for SubcompositorHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSubcompositor>) {
        slf.delete_id();
    }

    fn handle_get_subsurface(
        &mut self, _slf: &Rc<WlSubcompositor>, id: &Rc<WlSubsurface>, surface: &Rc<WlSurface>,
        parent: &Rc<WlSurface>,
    ) {
        let surface_id = surface.unique_id();
        let parent_id = parent.unique_id();
        self.state
            .borrow_mut()
            .assign_subsurface(surface_id, parent_id);

        id.set_forward_to_server(false);
        id.set_handler(SubsurfaceHandler {
            state: Rc::clone(&self.state),
            surface_id,
        });
    }
}

struct SubsurfaceHandler {
    state: Rc<RefCell<SharedState>>,
    surface_id: u64,
}

impl WlSubsurfaceHandler for SubsurfaceHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSubsurface>) {
        self.state
            .borrow_mut()
            .destroy_subsurface_role(self.surface_id);
        slf.delete_id();
    }

    fn handle_set_sync(&mut self, _slf: &Rc<WlSubsurface>) {
        self.state
            .borrow_mut()
            .set_subsurface_synchronized(self.surface_id, true);
    }

    fn handle_set_desync(&mut self, _slf: &Rc<WlSubsurface>) {
        self.state
            .borrow_mut()
            .set_subsurface_synchronized(self.surface_id, false);
    }
}

struct SurfaceHandler {
    shared: Rc<RefCell<SharedState>>,
}

impl WlSurfaceHandler for SurfaceHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSurface>) {
        self.shared.borrow_mut().destroy_surface(slf.unique_id());
        slf.delete_id();
    }

    fn handle_attach(
        &mut self, slf: &Rc<WlSurface>, buffer: Option<&Rc<WlBuffer>>, _x: i32, _y: i32,
    ) {
        self.shared
            .borrow_mut()
            .attach(slf.unique_id(), buffer.cloned());
    }

    fn handle_frame(&mut self, slf: &Rc<WlSurface>, callback: &Rc<WlCallback>) {
        callback.set_forward_to_server(false);
        self.shared
            .borrow_mut()
            .add_frame_callback(slf.unique_id(), Rc::clone(callback));
    }

    fn handle_commit(&mut self, slf: &Rc<WlSurface>) {
        self.shared.borrow_mut().commit_surface(slf.unique_id());
    }
}

impl SharedState {
    fn insert_surface(&mut self, surface: &Rc<WlSurface>) {
        let surface_id = surface.unique_id();
        for output in &self.outputs {
            surface.send_enter(output);
        }
        self.surfaces.insert(surface_id, Rc::clone(surface));
        self.surface_tree.insert_surface(surface_id);
        self.surface_runtime.entry(surface_id).or_default();
    }

    fn attach(&mut self, surface_id: u64, buffer: Option<Rc<WlBuffer>>) {
        if let Some(runtime) = self.surface_runtime.get_mut(&surface_id) {
            runtime.pending_buffer = Some(buffer);
        }
    }

    fn add_frame_callback(&mut self, surface_id: u64, callback: Rc<WlCallback>) {
        if let Some(runtime) = self.surface_runtime.get_mut(&surface_id) {
            runtime.pending_callbacks.push(callback);
        } else {
            self.complete_callbacks(vec![callback], 0);
        }
    }

    fn commit_surface(&mut self, surface_id: u64) {
        let Some(runtime) = self.surface_runtime.get_mut(&surface_id) else {
            return;
        };
        let commit = SurfaceCommit {
            buffer: runtime.pending_buffer.take().map(|buffer| {
                buffer.map(|buffer| AttachedBuffer {
                    buffer,
                    retained: false,
                })
            }),
            callbacks: std::mem::take(&mut runtime.pending_callbacks),
        };

        if self.surface_tree.is_effectively_synchronized(surface_id) {
            self.latch_commit(surface_id, commit);
            return;
        }

        let mut commits = vec![(surface_id, commit)];
        self.collect_latched_descendants(surface_id, &mut commits);
        self.apply_commits(commits);
    }

    fn latch_commit(&mut self, surface_id: u64, mut commit: SurfaceCommit) {
        if let Some(mut new_buffer) = commit.buffer.take() {
            if let Some(attached) = new_buffer.as_mut() {
                self.retain_attachment(attached);
            }
            let previous = {
                let Some(runtime) = self.surface_runtime.get_mut(&surface_id) else {
                    if let Some(attached) = new_buffer {
                        self.retire_attachment(attached);
                    }
                    return;
                };
                runtime
                    .latched
                    .get_or_insert_with(SurfaceCommit::default)
                    .buffer
                    .replace(new_buffer)
            };
            if let Some(Some(previous)) = previous {
                self.retire_attachment(previous);
            }
        }

        if !commit.callbacks.is_empty() {
            let Some(runtime) = self.surface_runtime.get_mut(&surface_id) else {
                self.complete_callbacks(commit.callbacks, 0);
                return;
            };
            runtime
                .latched
                .get_or_insert_with(SurfaceCommit::default)
                .callbacks
                .append(&mut commit.callbacks);
        }
    }

    fn collect_latched_descendants(
        &mut self, parent: u64, commits: &mut Vec<(u64, SurfaceCommit)>,
    ) {
        for descendant in self.surface_tree.descendants(parent) {
            if let Some(commit) = self
                .surface_runtime
                .get_mut(&descendant)
                .and_then(|runtime| runtime.latched.take())
            {
                commits.push((descendant, commit));
            }
        }
    }

    fn set_subsurface_synchronized(&mut self, surface_id: u64, synchronized: bool) {
        if let Err(error) = self
            .surface_tree
            .set_subsurface_synchronized(surface_id, synchronized)
        {
            tracing::error!(
                ?error,
                surface_id,
                "wl-proxy-mpv: could not update subsurface synchronization"
            );
            return;
        }

        if !synchronized && !self.surface_tree.is_effectively_synchronized(surface_id) {
            let mut commits = Vec::new();
            if let Some(commit) = self
                .surface_runtime
                .get_mut(&surface_id)
                .and_then(|runtime| runtime.latched.take())
            {
                commits.push((surface_id, commit));
            }
            self.collect_latched_descendants(surface_id, &mut commits);
            if !commits.is_empty() {
                self.apply_commits(commits);
            }
        }
    }

    fn apply_commits(&mut self, mut commits: Vec<(u64, SurfaceCommit)>) {
        let previous_selected = self.surface_tree.selected_video_surface();
        let mut changed_surfaces = HashSet::new();

        for (_, commit) in &mut commits {
            if let Some(Some(attached)) = commit.buffer.as_mut() {
                self.retain_attachment(attached);
            }
        }
        for (surface_id, commit) in &mut commits {
            if let Some(buffer) = commit.buffer.take() {
                changed_surfaces.insert(*surface_id);
                self.apply_surface_buffer(*surface_id, buffer);
            }
        }

        let selected = self.surface_tree.selected_video_surface();
        let action = content_action(
            previous_selected,
            selected,
            selected.is_some_and(|surface_id| changed_surfaces.contains(&surface_id)),
        );

        let callback_target = selected.or(previous_selected.filter(|_| selected.is_none()));
        let mut selected_callbacks = Vec::new();
        for (surface_id, commit) in &mut commits {
            let callbacks = std::mem::take(&mut commit.callbacks);
            if callback_target == Some(*surface_id) {
                selected_callbacks.extend(callbacks);
            } else {
                self.complete_callbacks(callbacks, 0);
            }
        }
        let frame_callbacks = self.register_callbacks(selected_callbacks);
        self.publish_content_action(action, callback_target, frame_callbacks);
    }

    fn retain_attachment(&mut self, attached: &mut AttachedBuffer) {
        if !attached.retained && self.retain_buffer(attached.buffer.unique_id()) {
            attached.retained = true;
        }
    }

    fn retire_attachment(&mut self, attached: AttachedBuffer) {
        if attached.retained {
            self.retire_buffer(attached.buffer.unique_id());
        }
    }

    fn apply_surface_buffer(&mut self, surface_id: u64, attachment: Option<AttachedBuffer>) {
        let new_current = attachment.and_then(|attached| {
            let current = if attached.retained {
                self.current_content(attached.buffer.unique_id())
            } else {
                None
            };
            match current {
                Some(current) => {
                    if !current.retains_buffer {
                        self.retire_attachment(attached);
                    }
                    Some(current)
                }
                None => {
                    self.retire_attachment(attached);
                    None
                }
            }
        });
        let metadata = new_current.as_ref().map(|current| current.metadata);
        let Some(runtime) = self.surface_runtime.get_mut(&surface_id) else {
            if let Some(new_current) = new_current {
                self.retire_current(new_current);
            }
            return;
        };
        let previous = std::mem::replace(&mut runtime.current, new_current);
        self.surface_tree.set_committed_buffer(surface_id, metadata);
        if let Some(previous) = previous {
            self.retire_current(previous);
        }
    }

    fn retire_current(&mut self, current: CurrentContent) {
        if current.retains_buffer {
            self.retire_buffer(current.buffer_id);
        }
    }

    fn current_content(&self, buffer_id: u64) -> Option<CurrentContent> {
        if let Some(info) = self.buffer_info.get(&buffer_id) {
            return Some(CurrentContent {
                buffer_id,
                metadata: info.metadata(buffer_id),
                retains_buffer: true,
                payload: CurrentPayload::Dmabuf,
            });
        }
        let info = self.shm_buffer_info.get(&buffer_id)?;
        Some(CurrentContent {
            buffer_id,
            metadata: info.metadata(buffer_id),
            retains_buffer: false,
            payload: CurrentPayload::Shm(info.snapshot()?),
        })
    }

    fn capture_current(&mut self, surface_id: u64) -> Option<CapturedFrame> {
        let current = self
            .surface_runtime
            .get(&surface_id)?
            .current
            .as_ref()?
            .clone();
        match current.payload {
            CurrentPayload::Dmabuf => {
                let frame_data = self.buffer_info.get(&current.buffer_id)?.frame_data()?;
                let use_token = self.begin_buffer_use(current.buffer_id)?;
                Some(CapturedFrame::Dmabuf(frame_data.into_frame(use_token)))
            }
            CurrentPayload::Shm(snapshot) => Some(CapturedFrame::Shm(snapshot.to_frame())),
        }
    }

    fn publish_content_action(
        &mut self, action: ContentAction, callback_target: Option<u64>,
        frame_callbacks: Option<FrameCallbacks>,
    ) {
        match action {
            ContentAction::PublishCurrent(surface_id) => {
                let content = self
                    .capture_current(surface_id)
                    .map(SurfaceContentUpdate::Frame)
                    .unwrap_or(SurfaceContentUpdate::Clear);
                self.publish_update(surface_id, content, frame_callbacks);
            }
            ContentAction::Clear(surface_id) => {
                self.publish_update(surface_id, SurfaceContentUpdate::Clear, frame_callbacks);
            }
            ContentAction::None => {
                if let (Some(surface_id), Some(frame_callbacks)) =
                    (callback_target, frame_callbacks)
                {
                    self.publish_update(
                        surface_id,
                        SurfaceContentUpdate::Unchanged,
                        Some(frame_callbacks),
                    );
                }
            }
        }
    }

    fn register_callbacks(&mut self, callbacks: Vec<Rc<WlCallback>>) -> Option<FrameCallbacks> {
        if callbacks.is_empty() {
            return None;
        }

        let mut callback_batch_id = self.next_callback_batch_id;
        while self.frame_callbacks.contains_key(&callback_batch_id) {
            callback_batch_id = callback_batch_id.wrapping_add(1);
        }
        self.next_callback_batch_id = callback_batch_id.wrapping_add(1);
        self.frame_callbacks.insert(callback_batch_id, callbacks);
        Some(FrameCallbacks {
            callback_batch_id: Some(callback_batch_id),
            event_tx: self.event_tx.clone(),
        })
    }

    fn complete_callbacks(&mut self, callbacks: Vec<Rc<WlCallback>>, time_ms: u32) {
        if let Some(callbacks) = self.register_callbacks(callbacks) {
            callbacks.done(time_ms);
        }
    }

    fn publish_update(
        &mut self, surface_id: u64, content: SurfaceContentUpdate,
        frame_callbacks: Option<FrameCallbacks>,
    ) {
        let update = SurfaceUpdate {
            generation: self.generation,
            load_id: self.load_id.load(std::sync::atomic::Ordering::Acquire),
            surface_id,
            content,
            frame_callbacks,
        };
        if let Err(error) = self.frame_tx.send(update) {
            let mut update = error.0;
            if let Some(callbacks) = update.frame_callbacks.take() {
                callbacks.done(0);
            }
        }
    }

    pub(super) fn assign_xdg_root(&mut self, surface_id: u64) {
        let previous_selected = self.surface_tree.selected_video_surface();
        if let Err(error) = self.surface_tree.assign_xdg_root(surface_id) {
            tracing::error!(
                ?error,
                surface_id,
                "wl-proxy-mpv: could not assign xdg root role"
            );
            return;
        }
        let selected = self.surface_tree.selected_video_surface();
        self.publish_content_action(
            content_action(previous_selected, selected, false),
            selected,
            None,
        );
    }

    fn assign_subsurface(&mut self, surface_id: u64, parent_id: u64) {
        let previous_selected = self.surface_tree.selected_video_surface();
        if let Err(error) = self.surface_tree.assign_subsurface(surface_id, parent_id) {
            tracing::error!(
                ?error,
                surface_id,
                parent_id,
                "wl-proxy-mpv: invalid subsurface relationship"
            );
            return;
        }
        let selected = self.surface_tree.selected_video_surface();
        self.publish_content_action(
            content_action(previous_selected, selected, false),
            selected,
            None,
        );
    }

    pub(super) fn clear_surface_role(&mut self, surface_id: u64) {
        let previous_selected = self.surface_tree.selected_video_surface();
        self.discard_latched_subtree(surface_id);
        self.clear_current_subtree(surface_id);
        self.surface_tree.clear_role(surface_id);
        let selected = self.surface_tree.selected_video_surface();
        self.publish_content_action(
            content_action(previous_selected, selected, false),
            selected,
            None,
        );
    }

    fn discard_latched_subtree(&mut self, surface_id: u64) {
        let mut surfaces = vec![surface_id];
        surfaces.extend(self.surface_tree.descendants(surface_id));
        for surface_id in surfaces {
            if let Some(latched) = self
                .surface_runtime
                .get_mut(&surface_id)
                .and_then(|runtime| runtime.latched.take())
            {
                if let Some(Some(attached)) = latched.buffer {
                    self.retire_attachment(attached);
                }
                self.complete_callbacks(latched.callbacks, 0);
            }
        }
    }

    fn clear_current_subtree(&mut self, surface_id: u64) {
        let mut surfaces = vec![surface_id];
        surfaces.extend(self.surface_tree.descendants(surface_id));
        for surface_id in surfaces {
            let current = self
                .surface_runtime
                .get_mut(&surface_id)
                .and_then(|runtime| runtime.current.take());
            self.surface_tree.set_committed_buffer(surface_id, None);
            if let Some(current) = current {
                self.retire_current(current);
            }
        }
    }

    fn destroy_subsurface_role(&mut self, surface_id: u64) {
        self.clear_surface_role(surface_id);
    }

    fn destroy_surface(&mut self, surface_id: u64) {
        let previous_selected = self.surface_tree.selected_video_surface();
        self.discard_latched_subtree(surface_id);
        self.clear_current_subtree(surface_id);
        if let Some(runtime) = self.surface_runtime.remove(&surface_id) {
            self.complete_callbacks(runtime.pending_callbacks, 0);
        }
        self.surface_tree.remove_surface(surface_id);
        self.surfaces.remove(&surface_id);
        self.viewport_states.remove(&surface_id);
        let selected = self.surface_tree.selected_video_surface();
        self.publish_content_action(
            content_action(previous_selected, selected, false),
            selected,
            None,
        );
    }
}
