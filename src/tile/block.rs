use super::Tile;
use std::ops::Index;

#[derive(Debug)]
pub struct Block<'a> {
    x: u32,
    y: u32,
    tiles: Vec<Tile<'a>>,
    width: usize,
}

impl<'a> Block<'a> {
    pub fn new(width: usize, (x, y): (u32, u32)) -> Self {
        Self {
            x,
            y,
            tiles: vec![],
            width,
        }
    }

    pub fn add_tile(&mut self, tile: Tile<'a>) {
        self.tiles.push(tile)
    }

    pub fn tiles(&self) -> &Vec<Tile<'a>> {
        &self.tiles
    }

    pub fn width(&self) -> usize {
        self.width
    }

    /// Only use after all tiles have been inserted!
    pub fn height(&self) -> usize {
        // Should not trip if block has been fully built
        debug_assert_eq!(self.tiles.len() % self.width, 0);
        self.tiles.len() / self.width
    }
}

impl<'a> Index<(usize, usize)> for Block<'a> {
    type Output = Tile<'a>;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.tiles[x + y * self.width]
    }
}
