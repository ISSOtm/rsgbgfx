use crate::tile::Color;
use std::convert::TryInto;

#[derive(Debug)]
pub struct Palette {
    colors: Vec<[Color; 4]>,
    nb_colors: u16,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            nb_colors: 0,
        }
    }

    /// Append a new color.
    /// The amount of 4-color palettes is capped in the 16-bit range, exceeding it is the Err() case.
    pub fn push(&mut self, color: Color) -> Result<(), ()> {
        if self.nb_colors % 4 == 0 {
            // Keep the amount of palettes in the u16 range
            if self.colors.len() == 65536 {
                return Err(());
            }
            self.colors
                .resize_with(self.colors.len() + 1, Default::default);
        }
        self.colors.last_mut().unwrap()[usize::from(self.nb_colors % 4)] = color;
        self.nb_colors += 1;
        Ok(())
    }

    pub fn nb_colors(&self) -> u16 {
        self.nb_colors
    }
}
