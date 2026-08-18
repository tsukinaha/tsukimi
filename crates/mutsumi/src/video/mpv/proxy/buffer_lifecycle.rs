use std::collections::HashMap;

pub type BufferId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferAction {
    SendRelease(BufferId),
    Purge(BufferId),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct BufferState {
    retained_refs: usize,
    in_flight_uses: usize,
    client_alive: bool,
    cycle_active: bool,
}

#[derive(Debug, Default)]
pub struct BufferLifecycle {
    buffers: HashMap<BufferId, BufferState>,
}

impl BufferLifecycle {
    pub fn register(&mut self, buffer_id: BufferId) {
        self.buffers.insert(
            buffer_id,
            BufferState {
                client_alive: true,
                ..BufferState::default()
            },
        );
    }

    pub fn retain(&mut self, buffer_id: BufferId) -> bool {
        let Some(state) = self.buffers.get_mut(&buffer_id) else {
            return false;
        };
        if !state.client_alive {
            return false;
        }
        state.retained_refs = state
            .retained_refs
            .checked_add(1)
            .expect("buffer retained reference count overflow");
        state.cycle_active = true;
        true
    }

    pub fn retire(&mut self, buffer_id: BufferId) -> Option<BufferAction> {
        let state = self.buffers.get_mut(&buffer_id)?;
        if state.retained_refs == 0 {
            return None;
        }
        state.retained_refs -= 1;
        self.finish_if_idle(buffer_id)
    }

    pub fn begin_use(&mut self, buffer_id: BufferId) -> bool {
        let Some(state) = self.buffers.get_mut(&buffer_id) else {
            return false;
        };
        if state.retained_refs == 0 {
            return false;
        }
        state.in_flight_uses = state
            .in_flight_uses
            .checked_add(1)
            .expect("buffer in-flight use count overflow");
        state.cycle_active = true;
        true
    }

    pub fn end_use(&mut self, buffer_id: BufferId) -> Option<BufferAction> {
        let state = self.buffers.get_mut(&buffer_id)?;
        if state.in_flight_uses == 0 {
            return None;
        }
        state.in_flight_uses -= 1;
        self.finish_if_idle(buffer_id)
    }

    pub fn client_destroyed(&mut self, buffer_id: BufferId) -> Option<BufferAction> {
        let state = self.buffers.get_mut(&buffer_id)?;
        state.client_alive = false;
        state.cycle_active = false;
        self.finish_if_idle(buffer_id)
    }

    fn finish_if_idle(&mut self, buffer_id: BufferId) -> Option<BufferAction> {
        let state = self.buffers.get_mut(&buffer_id)?;
        if state.retained_refs != 0 || state.in_flight_uses != 0 {
            return None;
        }
        if !state.client_alive {
            self.buffers.remove(&buffer_id);
            return Some(BufferAction::Purge(buffer_id));
        }
        if state.cycle_active {
            state.cycle_active = false;
            return Some(BufferAction::SendRelease(buffer_id));
        }
        None
    }
}
