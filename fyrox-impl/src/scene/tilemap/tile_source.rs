// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    core::{algebra::Vector2, reflect::prelude::*, visitor::prelude::*},
    rand::{seq::IteratorRandom, thread_rng},
};
use fxhash::FxHashMap;
use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use super::*;

/// The type of coordinates stored in a a [TileDefinitionHandle].
pub type PalettePosition = Vector2<i16>;

#[inline]
fn try_position(source: Vector2<i32>) -> Option<PalettePosition> {
    Some(PalettePosition::new(
        source.x.try_into().ok()?,
        source.y.try_into().ok()?,
    ))
}

#[inline]
fn position_to_vector(source: PalettePosition) -> Vector2<i32> {
    source.map(|x| x as i32)
}

/// A 2D grid that contains tile data.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct TileGridMap<V>(FxHashMap<Vector2<i32>, V>);

impl<V: Visit + Default> Visit for TileGridMap<V> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl<V> Deref for TileGridMap<V> {
    type Target = FxHashMap<Vector2<i32>, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> DerefMut for TileGridMap<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Position of a tile definition within some tile set
#[derive(Eq, PartialEq, Clone, Copy, Default, Hash, Reflect, Visit)]
pub struct TileDefinitionHandle {
    /// Position of the tile's page
    pub page: PalettePosition,
    /// Position of the tile definition within the page
    pub tile: PalettePosition,
}

impl Display for TileDefinitionHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({},{}):({},{})",
            self.page.x, self.page.y, self.tile.x, self.tile.y
        )
    }
}

impl Debug for TileDefinitionHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TileDefinitionHandle({},{};{},{})",
            self.page.x, self.page.y, self.tile.x, self.tile.y
        )
    }
}

impl FromStr for TileDefinitionHandle {
    type Err = TileDefinitionHandleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(TileDefinitionHandleParseError)
    }
}

/// An syntax error in parsing a TileDefinitionHandle from a string.
#[derive(Debug)]
pub struct TileDefinitionHandleParseError;

impl Display for TileDefinitionHandleParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tile definition handle parse failure")
    }
}

impl Error for TileDefinitionHandleParseError {}

impl TileDefinitionHandle {
    /// Attempt to construct a handle for the given page and tile positions.
    /// Handles use a pair of i16 vectors, so that the total is 64 bits.
    /// If the given vectors are outside of the range that can be represented as i16 coordinates,
    /// then None is returned.
    pub fn try_new(page: Vector2<i32>, tile: Vector2<i32>) -> Option<Self> {
        Some(Self {
            page: try_position(page)?,
            tile: try_position(tile)?,
        })
    }
    /// Construct a handle directly from coordinates. This is intended for cases
    /// where certain tile handles may need to be hard-coded as having special significance.
    pub const fn new(page_x: i16, page_y: i16, tile_x: i16, tile_y: i16) -> Self {
        Self {
            page: PalettePosition::new(page_x, page_y),
            tile: PalettePosition::new(tile_x, tile_y),
        }
    }
    /// Extracts the page coordinates and converts them to an i32 vector.
    pub fn page(&self) -> Vector2<i32> {
        position_to_vector(self.page)
    }
    /// Extracts the tile coordinates and converts them to an i32 vector.
    pub fn tile(&self) -> Vector2<i32> {
        position_to_vector(self.tile)
    }
    pub fn parse(s: &str) -> Option<Self> {
        let mut iter = s
            .split(|c: char| c != '-' && !c.is_ascii_digit())
            .filter(|w| !w.is_empty());
        let a: i16 = iter.next()?.parse().ok()?;
        let b: i16 = iter.next()?.parse().ok()?;
        let c: i16 = iter.next()?.parse().ok()?;
        let d: i16 = iter.next()?.parse().ok()?;
        if iter.next().is_some() {
            None
        } else {
            Some(Self::new(a, b, c, d))
        }
    }
}

/// A region of tiles to be filled from some source of tiles.
#[derive(Debug, Default, Clone)]
pub struct TileRegion {
    /// The position to put the (0,0) tile of the tile source.
    /// If `origin` is not within `bounds` then the (0,0) tile will not actually be used.
    pub origin: Vector2<i32>,
    /// The area to fill.
    pub bounds: OptionTileRect,
}

impl TileRegion {
    /// Construct a region with its origin in one of the four corners of the given bounds.
    /// The corner of the origin is based on the given direction.
    pub fn from_bounds_and_direction(bounds: OptionTileRect, direction: Vector2<i32>) -> Self {
        let Some(bounds) = *bounds else {
            return Self::default();
        };
        let x0 = if direction.x <= 0 {
            bounds.left_bottom_corner().x
        } else {
            bounds.right_top_corner().x
        };
        let y0 = if direction.y <= 0 {
            bounds.left_bottom_corner().y
        } else {
            bounds.right_top_corner().y
        };
        Self {
            origin: Vector2::new(x0, y0),
            bounds: bounds.clone().into(),
        }
    }
    /// Construct a region with `bounds` that contain `origin` and `end`.
    pub fn from_points(origin: Vector2<i32>, end: Vector2<i32>) -> Self {
        Self {
            origin,
            bounds: OptionTileRect::from_points(origin, end),
        }
    }
    pub fn with_bounds(mut self, bounds: OptionTileRect) -> Self {
        self.bounds = bounds;
        self
    }
    /// Reduce the size of `bounds` by deflating them by the given amounts.
    pub fn deflate(mut self, dw: i32, dh: i32) -> Self {
        self.bounds = self.bounds.deflate(dw, dh);
        self
    }
    /// Iterator over `(target, source)` pairs where `target` is the position to put the tile
    /// and `source` is the position to get the tile from within the tile source.
    /// Every position within `bounds` will appear once as the `target`.
    /// If `origin` is within `bounds`, then `(origin, (0,0))` will be produced by the iterator.
    pub fn iter(&self) -> impl Iterator<Item = (Vector2<i32>, Vector2<i32>)> + '_ {
        self.bounds.iter().map(|p| (p, p - self.origin))
    }
}

/// A trait for types that can produce a TileDefinitionHandle upon demand,
/// for use with drawing on tilemaps.
pub trait TileSource {
    /// The transformation that should be applied to the tiles before they are written.
    fn transformation(&self) -> OrthoTransformation;
    /// Produce a tile definition handle for the given position. If an area of multiple
    /// tiles is being filled, then the given position represents where the tile
    /// will go within the area.
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle>;
}

/// A tile source that always produces the same tile.
#[derive(Clone, Debug)]
pub struct SingleTileSource(pub OrthoTransformation, pub TileDefinitionHandle);

impl TileSource for SingleTileSource {
    fn transformation(&self) -> OrthoTransformation {
        self.0
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        Some(self.1)
    }
}

/// A tile source that produces a random tile from the included set of tiles.
pub struct RandomTileSource<'a>(pub &'a Stamp);

impl<'a> TileSource for RandomTileSource<'a> {
    fn transformation(&self) -> OrthoTransformation {
        self.0.transformation()
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.0.values().choose(&mut thread_rng()).copied()
    }
}

/// A tile source that produces a random tile from the included set of tiles.
pub struct PartialRandomTileSource<'a>(pub &'a Stamp, pub OptionTileRect);

impl<'a> TileSource for PartialRandomTileSource<'a> {
    fn transformation(&self) -> OrthoTransformation {
        self.0.transformation()
    }
    fn get_at(&self, _position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        let pos = self.1.iter().choose(&mut thread_rng())?;
        self.0.get_at(pos)
    }
}

/// A tile source that adapts another source so that it infinitely repeats the tiles
/// within the given rect.
pub struct RepeatTileSource<'a, S> {
    /// The tiles to repeat
    pub source: &'a S,
    /// The region within the stamp to repeat
    pub region: TileRegion,
}

impl<'a, S: TileSource> TileSource for RepeatTileSource<'a, S> {
    fn transformation(&self) -> OrthoTransformation {
        self.source.transformation()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        let rect = (*self.region.bounds)?;
        let rect_pos = rect.position;
        let size = rect.size;
        let pos = position + self.region.origin - rect_pos;
        let x = pos.x.rem_euclid(size.x);
        let y = pos.y.rem_euclid(size.y);
        self.source.get_at(Vector2::new(x, y) + rect_pos)
    }
}

/// A set of tiles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tiles(TileGridMap<TileDefinitionHandle>);

/// A set of tiles and a transformation, which represents the tiles that the user has selected
/// to draw with.
#[derive(Clone, Debug, Default, Visit)]
pub struct Stamp(OrthoTransformation, OrthoTransformMap<TileDefinitionHandle>);

impl TileSource for Tiles {
    fn transformation(&self) -> OrthoTransformation {
        OrthoTransformation::default()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.get(&position).copied()
    }
}

impl Visit for Tiles {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Deref for Tiles {
    type Target = TileGridMap<TileDefinitionHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileSource for Stamp {
    fn transformation(&self) -> OrthoTransformation {
        self.0
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.1.get(position).copied()
    }
}

impl Stamp {
    /// Create a repeating tile source from this stamp to repeat from `start` to `end.`
    pub fn repeat(&self, start: Vector2<i32>, end: Vector2<i32>) -> RepeatTileSource<Stamp> {
        let bounds = self.bounding_rect();
        RepeatTileSource {
            source: self,
            region: TileRegion::from_bounds_and_direction(bounds, start - end),
        }
    }

    /// True if this stamp contains no tiles.
    pub fn is_empty(&self) -> bool {
        self.1.is_empty()
    }
    /// Turn this stamp into an empty stamp.
    pub fn clear(&mut self) {
        self.1.clear();
        self.0 = OrthoTransformation::identity();
    }
    /// Clear this stamp and fill it with the given tiles.
    /// The tiles are moved so that their center is (0,0).
    /// The transform is set to identity.
    pub fn build<I: Iterator<Item = (Vector2<i32>, TileDefinitionHandle)> + Clone>(
        &mut self,
        source: I,
    ) {
        self.clear();
        let mut rect = OptionTileRect::default();
        for (p, _) in source.clone() {
            rect.push(p);
        }
        let Some(rect) = *rect else {
            return;
        };
        let center = rect.center();
        for (p, h) in source {
            self.insert(p - center, h);
        }
    }
    /// Rotate the stamp by the given number of 90-degree turns.
    pub fn rotate(&mut self, amount: i8) {
        self.0 = self.0.rotated(amount);
        self.1 = std::mem::take(&mut self.1).rotated(amount);
    }
    /// Flip along the x axis.
    pub fn x_flip(&mut self) {
        self.0 = self.0.x_flipped();
        self.1 = std::mem::take(&mut self.1).x_flipped();
    }
    /// Flip along the y axis.
    pub fn y_flip(&mut self) {
        self.0 = self.0.y_flipped();
        self.1 = std::mem::take(&mut self.1).y_flipped();
    }
}

impl Deref for Stamp {
    type Target = OrthoTransformMap<TileDefinitionHandle>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl DerefMut for Stamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}

impl Tiles {
    /// Construct a new tile set from the given hash map.
    pub fn new(source: TileGridMap<TileDefinitionHandle>) -> Self {
        Self(source)
    }
    /// Apply the updates specified in the given `TileUpdates` and modify it so that it
    /// contains the tiles require to undo the change. Calling `swap_tiles` twice with the same
    /// `TileUpdates` object will do the changes and then undo them, leaving the tiles unchanged in the end.
    pub fn swap_tiles(&mut self, updates: &mut TilesUpdate) {
        for (k, v) in updates.iter_mut() {
            swap_hash_map_entry(self.entry(*k), v);
        }
    }
    /// Calculates bounding rectangle in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for position in self.keys() {
            result.push(*position);
        }
        result
    }

    /// Clears the tile container.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}
