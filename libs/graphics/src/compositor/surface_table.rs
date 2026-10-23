use crate::{
    compositor::surface::{RenderCanvas, Surface, SurfaceBacking, SurfaceId, SurfaceInfo},
    error::DisplayError,
};

pub const MAX_SURFACES: usize = 16;

#[derive(Debug)]
pub struct SurfaceTable {
    // stores surfaces indexed by surface_id. 1-indexed because surface 0 is reserved for the primary display
    surfaces: [Option<Surface>; MAX_SURFACES],
}

pub type OwnerId = u32;

impl SurfaceTable {
    // registers a surface with a backing and canvas, using an id. reutns
    pub fn create_surface(
        &mut self,
        owner: OwnerId,
        canvas: RenderCanvas,
        backing: SurfaceBacking,
    ) -> Result<SurfaceId, DisplayError> {
        Self::verify_backing(&canvas, &backing)?;
        let id = self.next_id()?;
        let info = SurfaceInfo {
            owner_id: owner,
            surface_id: id as u32,
        };
        let surface = Surface::new(info, canvas, backing);
        self.surfaces[id] = Some(surface);
        Ok(id as u32)
    }
    /// Deregisters the surface, freeing the id. Does not free the physical backing.
    pub fn destroy_surface(&mut self, surface_id: SurfaceId) -> Result<(), DisplayError> {
        let surface = self
            .surfaces
            .get_mut(surface_id as usize)
            .ok_or(DisplayError::InvalidSurface)?;
        *surface = None;
        Ok(())
    }
    pub fn next_id(&self) -> Result<usize, DisplayError> {
        let next = self
            .surfaces
            .iter()
            .position(|s| s.is_none())
            .ok_or(DisplayError::SurfacesExhausted)?;
        Ok(next)
    }
    pub fn surface(&self, surface_id: SurfaceId) -> Result<&Surface, DisplayError> {
        self.surfaces
            .get(surface_id as usize)
            .map(|s| s.as_ref().unwrap())
            .ok_or(DisplayError::InvalidSurface)
    }
    pub fn surface_mut(&mut self, surface_id: SurfaceId) -> Result<&mut Surface, DisplayError> {
        self.surfaces
            .get_mut(surface_id as usize)
            .map(|s| s.as_mut().unwrap())
            .ok_or(DisplayError::InvalidSurface)
    }
    /// Returns an iterator over all surface ids of surfaces with owner `owner`
    pub fn child_ids(&self, owner: OwnerId) -> impl Iterator<Item = SurfaceId> {
        self.surfaces
            .iter()
            .flatten()
            .filter(move |s| s.info.owner_id == owner)
            .map(|s| s.info.surface_id)
    }
    /// Helper to verify whether the backing can support the canvas. Returns [`Err`] if the backing doesnt match
    fn verify_backing(canvas: &RenderCanvas, backing: &SurfaceBacking) -> Result<(), DisplayError> {
        let min_size = canvas.buffer_size();
        let size: usize = backing.size as usize;
        if backing.size < min_size as u64 {
            return Err(DisplayError::InsufficientBacking(size, min_size));
        }
        Ok(())
    }
}
