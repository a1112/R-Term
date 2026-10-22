use std::{error::Error, io, num::NonZeroU32, sync::Arc};

use winit::{
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop, OwnedDisplayHandle},
    window::Window,
};

fn write_rgba_to_softbuffer(rgba: &[u8], target: &mut [u32]) -> io::Result<()> {
    let expected = target
        .len()
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bootstrap size overflow"))?;
    if rgba.len() < expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "RGBA bootstrap frame is smaller than the surface",
        ));
    }
    for (packed, pixel) in target.iter_mut().zip(rgba[..expected].chunks_exact(4)) {
        *packed = (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2]);
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct WindowBootstrapSurface {
    _context: softbuffer::Context<OwnedDisplayHandle>,
    surface: softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>,
    size: PhysicalSize<u32>,
}

impl WindowBootstrapSurface {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
        size: PhysicalSize<u32>,
    ) -> Result<Self, Box<dyn Error>> {
        let width = NonZeroU32::new(size.width)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "zero bootstrap width"))?;
        let height = NonZeroU32::new(size.height)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "zero bootstrap height"))?;
        let display = event_loop.owned_display_handle();
        let context = softbuffer::Context::new(display.clone())?;
        let mut surface = softbuffer::Surface::new(&context, window)?;
        surface.resize(width, height)?;
        Ok(Self {
            _context: context,
            surface,
            size,
        })
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), Box<dyn Error>> {
        let width = NonZeroU32::new(size.width)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "zero bootstrap width"))?;
        let height = NonZeroU32::new(size.height)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "zero bootstrap height"))?;
        self.surface.resize(width, height)?;
        self.size = size;
        Ok(())
    }

    pub(crate) fn present_rgba(&mut self, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
        let expected = usize::try_from(self.size.width)
            .ok()
            .and_then(|width| {
                usize::try_from(self.size.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "bootstrap size overflow")
            })?;
        if rgba.len() < expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RGBA bootstrap frame is smaller than the surface",
            )
            .into());
        }
        let mut buffer = self.surface.buffer_mut()?;
        write_rgba_to_softbuffer(&rgba[..expected], &mut buffer)?;
        buffer.present()?;
        Ok(())
    }

    pub(crate) fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_rgba_pixels_directly_into_softbuffer_order() {
        let rgba = [
            0x11, 0x22, 0x33, 0xff, // red, green, blue, alpha
            0xaa, 0xbb, 0xcc, 0x00,
        ];
        let mut softbuffer_pixels = [0; 2];

        write_rgba_to_softbuffer(&rgba, &mut softbuffer_pixels).unwrap();

        assert_eq!(softbuffer_pixels, [0x0011_2233, 0x00aa_bbcc]);
    }
}
