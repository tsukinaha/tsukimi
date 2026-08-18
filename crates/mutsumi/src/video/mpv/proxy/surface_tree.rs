use std::collections::{
    HashMap,
    HashSet,
};

pub type SurfaceId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRole {
    Unassigned,
    XdgRoot,
    Subsurface {
        parent: SurfaceId,
        synchronized: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferMetadata {
    pub buffer_id: u64,
    pub width: u32,
    pub height: u32,
}

impl BufferMetadata {
    fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceTreeError {
    UnknownSurface(SurfaceId),
    RoleAlreadyAssigned(SurfaceId),
    InvalidParent,
}

#[derive(Debug)]
struct SurfaceNode {
    role: SurfaceRole,
    buffer: Option<BufferMetadata>,
}

#[derive(Debug, Default)]
pub struct SurfaceTree {
    nodes: HashMap<SurfaceId, SurfaceNode>,
}

impl SurfaceTree {
    pub fn insert_surface(&mut self, surface_id: SurfaceId) {
        self.nodes.entry(surface_id).or_insert(SurfaceNode {
            role: SurfaceRole::Unassigned,
            buffer: None,
        });
    }

    pub fn remove_surface(&mut self, surface_id: SurfaceId) {
        self.nodes.remove(&surface_id);
        for node in self.nodes.values_mut() {
            if matches!(node.role, SurfaceRole::Subsurface { parent, .. } if parent == surface_id) {
                node.role = SurfaceRole::Unassigned;
            }
        }
    }

    pub fn assign_xdg_root(&mut self, surface_id: SurfaceId) -> Result<(), SurfaceTreeError> {
        let node = self
            .nodes
            .get_mut(&surface_id)
            .ok_or(SurfaceTreeError::UnknownSurface(surface_id))?;
        if node.role != SurfaceRole::Unassigned {
            return Err(SurfaceTreeError::RoleAlreadyAssigned(surface_id));
        }
        node.role = SurfaceRole::XdgRoot;
        Ok(())
    }

    pub fn assign_subsurface(
        &mut self, surface_id: SurfaceId, parent: SurfaceId,
    ) -> Result<(), SurfaceTreeError> {
        if surface_id == parent || !self.nodes.contains_key(&parent) {
            return Err(SurfaceTreeError::InvalidParent);
        }
        let node = self
            .nodes
            .get(&surface_id)
            .ok_or(SurfaceTreeError::UnknownSurface(surface_id))?;
        if node.role != SurfaceRole::Unassigned {
            return Err(SurfaceTreeError::RoleAlreadyAssigned(surface_id));
        }

        let mut current = Some(parent);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if id == surface_id || !visited.insert(id) {
                return Err(SurfaceTreeError::InvalidParent);
            }
            current = match self.nodes.get(&id).map(|node| node.role) {
                Some(SurfaceRole::Subsurface { parent, .. }) => Some(parent),
                _ => None,
            };
        }

        self.nodes.get_mut(&surface_id).unwrap().role = SurfaceRole::Subsurface {
            parent,
            synchronized: true,
        };
        Ok(())
    }

    pub fn clear_role(&mut self, surface_id: SurfaceId) {
        if let Some(node) = self.nodes.get_mut(&surface_id) {
            node.role = SurfaceRole::Unassigned;
        }
    }

    pub fn set_subsurface_synchronized(
        &mut self, surface_id: SurfaceId, synchronized: bool,
    ) -> Result<(), SurfaceTreeError> {
        let node = self
            .nodes
            .get_mut(&surface_id)
            .ok_or(SurfaceTreeError::UnknownSurface(surface_id))?;
        let SurfaceRole::Subsurface { parent, .. } = node.role else {
            return Err(SurfaceTreeError::RoleAlreadyAssigned(surface_id));
        };
        node.role = SurfaceRole::Subsurface {
            parent,
            synchronized,
        };
        Ok(())
    }

    pub fn is_effectively_synchronized(&self, surface_id: SurfaceId) -> bool {
        let mut current = Some(surface_id);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                return true;
            }
            match self.nodes.get(&id).map(|node| node.role) {
                Some(SurfaceRole::Subsurface {
                    parent,
                    synchronized,
                }) => {
                    if synchronized {
                        return true;
                    }
                    current = Some(parent);
                }
                _ => return false,
            }
        }
        false
    }

    pub fn descendants(&self, parent: SurfaceId) -> Vec<SurfaceId> {
        let mut descendants = Vec::new();
        let mut frontier = vec![parent];
        while let Some(parent) = frontier.pop() {
            let mut children: Vec<_> = self
                .nodes
                .iter()
                .filter_map(|(&surface_id, node)| {
                    matches!(node.role, SurfaceRole::Subsurface { parent: node_parent, .. } if node_parent == parent)
                        .then_some(surface_id)
                })
                .collect();
            children.sort_unstable();
            frontier.extend(children.iter().rev().copied());
            descendants.extend(children);
        }
        descendants
    }

    pub fn set_committed_buffer(&mut self, surface_id: SurfaceId, buffer: Option<BufferMetadata>) {
        if let Some(node) = self.nodes.get_mut(&surface_id) {
            node.buffer = buffer;
        }
    }

    fn xdg_root_and_depth(&self, surface_id: SurfaceId) -> Option<(SurfaceId, usize)> {
        let mut current = surface_id;
        let mut depth = 0;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                return None;
            }
            match self.nodes.get(&current)?.role {
                SurfaceRole::XdgRoot => return Some((current, depth)),
                SurfaceRole::Subsurface { parent, .. } => {
                    current = parent;
                    depth += 1;
                }
                SurfaceRole::Unassigned => return None,
            }
        }
    }

    pub fn selected_video_surface(&self) -> Option<SurfaceId> {
        let mut selected: Option<(SurfaceId, u64, usize)> = None;
        for (&surface_id, node) in &self.nodes {
            let Some(buffer) = node.buffer else {
                continue;
            };
            let Some((_root, depth)) = self.xdg_root_and_depth(surface_id) else {
                continue;
            };

            let area = buffer.area();
            if selected.is_none_or(|(selected_id, selected_area, selected_depth)| {
                area > selected_area
                    || (area == selected_area && depth > selected_depth)
                    || (area == selected_area
                        && depth == selected_depth
                        && surface_id < selected_id)
            }) {
                selected = Some((surface_id, area, depth));
            }
        }
        selected.map(|(surface_id, _, _)| surface_id)
    }
}
