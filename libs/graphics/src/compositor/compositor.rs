use crate::{
    compositor::{
        surface::{RenderCanvas, SurfaceBacking, SurfaceId, SurfaceInfo},
        surface_table::{OwnerId, SurfaceTable},
    },
    error::DisplayError,
    primitives::{color::ColorFormat, rect::ScreenRect},
};

pub struct CursorState {
    surface_id: SurfaceId,
    x: u32,
    y: u32,
}

pub struct Compositor {
    surfaces: SurfaceTable,
    next_pid: u32,
    cursor: CursorState,

    /* exposed gpu internal state */
    current_surface: SurfaceId,
    currect_rect: ScreenRect,
}

impl Compositor {
    /* Resource Management */
    /// creates a 2d surface with the provided width, height and color format, returning the surface id
    fn create_surface(
        &mut self,
        owner: OwnerId,
        width: usize,
        height: usize,
        format: ColorFormat,
        backing: SurfaceBacking,
    ) -> Result<SurfaceId, DisplayError> {
        let canvas = RenderCanvas::new(width, height, format);
        self.surfaces.create_surface(owner, canvas, backing)
    }
    /// Deregisters the surface with the given `surface_id`, freeing the slot. Does not deallocate the backing in physical memory
    fn destroy_surface(&mut self, surface_id: SurfaceId) -> Result<(), DisplayError> {
        self.surfaces.destroy_surface(surface_id)
    }
    /// Retuns [`Ok`] containing the [`SurfaceInfo`] of surface with id `surface_id`, or [`Err`] if the surface id is invalid
    fn get_surface_info(&self, surface_id: SurfaceId) -> Result<SurfaceInfo, DisplayError> {
        Ok(self.surfaces.surface(surface_id)?.info)
    }
    /// Shows the surface with the specified `surface_id`
    fn show(&mut self, surface_id: SurfaceId) -> Result<(), DisplayError> {
        if self.surfaces.surface(surface_id).is_err() {
            return Err(DisplayError::InvalidSurface);
        }
        self.current_surface = surface_id;
        Ok(())
    }
    /// Displays the surface with id: `surface_id`. If `damage` is specified then only the `damage` region of the resource will be updated
    fn present(
        &mut self,
        surface_id: SurfaceId,
        damage: Option<ScreenRect>,
    ) -> Result<(), DisplayError> {
        todo!()
    }

    fn wait(&mut self) -> Result<(), DisplayError> {
        todo!()
    }
    /// Sets the cursor's image and position. The cursor will be composited over the main buffer. If `surface_id` is specified then it will attempt to use the given surface as the cursor image.
    /// The cursor's surface dimensions will be limited to 64x64 pixels
    fn set_cursor(&mut self, image: Option<SurfaceId>, x: u32, y: u32) -> Result<(), DisplayError> {
        if let Some(id) = image {
            self.surfaces
                .surface(id)
                .map_err(|_| DisplayError::InvalidSurface)?;
            self.cursor.surface_id = id;
        }

        self.cursor.x = x;
        self.cursor.y = y;
        Ok(())
    }
}
