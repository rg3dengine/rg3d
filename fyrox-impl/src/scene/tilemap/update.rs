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

use super::*;
use crate::{
    core::{algebra::Vector2, color::Color, type_traits::prelude::*},
    material::MaterialResource,
};
use fxhash::FxHashMap;
use std::ops::{Deref, DerefMut};
use std::{
    borrow::Cow,
    collections::hash_map::{Entry, Keys},
};

struct BresenhamLineIter {
    dx: i32,
    dy: i32,
    x: i32,
    y: i32,
    error: i32,
    end_x: i32,
    is_steep: bool,
    y_step: i32,
}

impl BresenhamLineIter {
    fn new(start: Vector2<i32>, end: Vector2<i32>) -> BresenhamLineIter {
        let (mut x0, mut y0) = (start.x, start.y);
        let (mut x1, mut y1) = (end.x, end.y);

        let is_steep = (y1 - y0).abs() > (x1 - x0).abs();
        if is_steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }

        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;

        BresenhamLineIter {
            dx,
            dy: (y1 - y0).abs(),
            x: x0,
            y: y0,
            error: dx / 2,
            end_x: x1,
            is_steep,
            y_step: if y0 < y1 { 1 } else { -1 },
        }
    }
}

impl Iterator for BresenhamLineIter {
    type Item = Vector2<i32>;

    fn next(&mut self) -> Option<Vector2<i32>> {
        if self.x > self.end_x {
            None
        } else {
            let ret = if self.is_steep {
                Vector2::new(self.y, self.x)
            } else {
                Vector2::new(self.x, self.y)
            };

            self.x += 1;
            self.error -= self.dy;
            if self.error < 0 {
                self.y += self.y_step;
                self.error += self.dx;
            }

            Some(ret)
        }
    }
}

/// This represents a change to some pages of a tile set, without specifying which tile set.
#[derive(Clone, Debug, Default)]
pub struct TileSetUpdate(FxHashMap<TileDefinitionHandle, TileDataUpdate>);

impl Deref for TileSetUpdate {
    type Target = FxHashMap<TileDefinitionHandle, TileDataUpdate>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for TileSetUpdate {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A change of material for some tile. Either the material is being erased,
/// or it is being replaced by the given material.
#[derive(Debug, Clone)]
pub enum MaterialUpdate {
    /// This update is eliminating the material from the tile.
    Erase,
    /// This update is replacing the material of the tile.
    Replace(TileMaterialBounds),
}

/// This represents a change to a tile in some tile set.
#[derive(Clone, Debug, Default)]
pub enum TileDataUpdate {
    /// Remove this tile.
    #[default]
    Erase,
    /// This variant is for changing a material page tile.
    MaterialTile(TileData),
    /// This variant is for changing a freeform page tile.
    FreeformTile(TileDefinition),
    /// This variant is for changing the transform of a tile.
    /// This update must be applied to some cell of transform set page.
    /// It contains the new source tile for the transform cell.
    TransformSet(Option<TileDefinitionHandle>),
    /// This variant is for changing a tile's color.
    Color(Color),
    /// This variant is for changing a tile's property.
    Property(Uuid, Option<TileSetPropertyValue>),
    /// This variant is for changing some of a tile property's nine slices.
    PropertySlice(Uuid, [Option<i8>; 9]),
    /// This variant is for changing a tile's collider.
    Collider(Uuid, Option<TileCollider>),
    /// This variant is for changing a tile's material.
    Material(TileMaterialBounds),
}

impl TileDataUpdate {
    /// The handle that should be used in place of the given handle, if this update has changed
    /// the handle of a transform set tile.
    /// None is returned if no tile should be rendered.
    /// The given tile is returned if no change should be made.
    pub fn substitute_transform_handle(
        &self,
        source: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        if let TileDataUpdate::TransformSet(new_source) = self {
            *new_source
        } else {
            Some(source)
        }
    }
    /// The render data that should be used in place of the given render data, based on this update.
    /// None is returned if no tile should be rendered.
    pub fn modify_render<'a>(&self, source: &'a TileRenderData) -> Option<Cow<'a, TileRenderData>> {
        match self {
            TileDataUpdate::Erase => None,
            TileDataUpdate::MaterialTile(tile_data) => Some(Cow::Owned(TileRenderData {
                material_bounds: source.material_bounds.clone(),
                color: tile_data.color,
            })),
            TileDataUpdate::FreeformTile(def) => Some(Cow::Owned(TileRenderData {
                material_bounds: Some(def.material_bounds.clone()),
                color: def.data.color,
            })),
            TileDataUpdate::Color(color) => Some(Cow::Owned(TileRenderData {
                material_bounds: source.material_bounds.clone(),
                color: *color,
            })),
            TileDataUpdate::Material(material_bounds) => Some(Cow::Owned(TileRenderData {
                material_bounds: Some(material_bounds.clone()),
                color: source.color,
            })),
            _ => Some(Cow::Borrowed(source)),
        }
    }
    /// Remove `TileData` and turn this object into `Erase`, if this is a MaterialTile. Otherwise, panic.
    pub fn take_data(&mut self) -> TileData {
        match std::mem::take(self) {
            TileDataUpdate::MaterialTile(d) => d,
            _ => panic!(),
        }
    }
    /// Remove `TileDefinition` and turn this object into `Erase`, if this is a FreeformTile. Otherwise, panic.
    pub fn take_definition(&mut self) -> TileDefinition {
        match std::mem::take(self) {
            TileDataUpdate::FreeformTile(d) => d,
            _ => panic!(),
        }
    }
    /// Swap whatever value is in this tile update with the corresponding value in the given TileData.
    /// If this update is `Erase` then it has no data to swap, so panic.
    pub fn swap_with_data(&mut self, data: &mut TileData) {
        match self {
            TileDataUpdate::Erase => panic!(),
            TileDataUpdate::MaterialTile(tile_data) => std::mem::swap(tile_data, data),
            TileDataUpdate::FreeformTile(tile_definition) => {
                std::mem::swap(&mut tile_definition.data, data)
            }
            TileDataUpdate::Color(color) => std::mem::swap(color, &mut data.color),
            TileDataUpdate::Collider(uuid, value) => {
                swap_hash_map_entry(data.collider.entry(*uuid), value)
            }
            TileDataUpdate::Property(uuid, value) => {
                swap_hash_map_entry(data.properties.entry(*uuid), value)
            }
            TileDataUpdate::PropertySlice(uuid, value) => match data.properties.entry(*uuid) {
                Entry::Occupied(mut e) => {
                    if let TileSetPropertyValue::NineSlice(v0) = e.get_mut() {
                        for (v0, v1) in v0.iter_mut().zip(value.iter_mut()) {
                            if let Some(v1) = v1 {
                                std::mem::swap(v0, v1);
                            }
                        }
                    }
                }
                Entry::Vacant(e) => {
                    let _ = e.insert(TileSetPropertyValue::NineSlice(
                        value.map(|v| v.unwrap_or_default()),
                    ));
                    *self = TileDataUpdate::Property(*uuid, None);
                }
            },
            TileDataUpdate::TransformSet(_) => panic!(),
            TileDataUpdate::Material(_) => panic!(),
        }
    }
}

impl TileSetUpdate {
    /// Attempt to fill this TileSetUpdate based upon a TransTilesUpdate.
    /// The TransTilesUpdate contains only positions, transformations, and TileDefinitionHandles for the tiles that are to be written.
    /// In order to construct a TileSetUpdate, we use the given TileSet to copy tile bounds and tile definition data
    /// as appropriate for the kind of page we are updating.
    ///
    /// Nothing is done if the given page does not exist or if it is a Material page that cannot be written to.
    pub fn convert(&mut self, tiles: &TransTilesUpdate, tile_set: &TileSet, page: Vector2<i32>) {
        let Some(page_object) = tile_set.get_page(page) else {
            return;
        };
        match &page_object.source {
            TileSetPageSource::Material(_) => self.convert_material(tiles, page),
            TileSetPageSource::Freeform(_) => self.convert_freeform(tiles, tile_set, page),
            TileSetPageSource::TransformSet(_) => self.convert_transform(tiles, tile_set, page),
        }
    }
    fn convert_material(&mut self, tiles: &TransTilesUpdate, page: Vector2<i32>) {
        for (pos, value) in tiles.iter() {
            let Some(handle) = TileDefinitionHandle::try_new(page, *pos) else {
                continue;
            };
            if value.is_some() {
                self.insert(handle, TileDataUpdate::MaterialTile(TileData::default()));
            } else {
                self.insert(handle, TileDataUpdate::Erase);
            }
        }
    }
    fn convert_freeform(
        &mut self,
        tiles: &TransTilesUpdate,
        tile_set: &TileSet,
        page: Vector2<i32>,
    ) {
        for (pos, value) in tiles.iter() {
            let Some(handle) = TileDefinitionHandle::try_new(page, *pos) else {
                continue;
            };
            if let Some(def) = value.and_then(|(t, h)| tile_set.get_transformed_definition(t, h)) {
                self.insert(handle, TileDataUpdate::FreeformTile(def));
            } else {
                self.insert(handle, TileDataUpdate::Erase);
            }
        }
    }
    fn convert_transform(
        &mut self,
        tiles: &TransTilesUpdate,
        tile_set: &TileSet,
        page: Vector2<i32>,
    ) {
        for (pos, value) in tiles.iter() {
            let Some(target_handle) = TileDefinitionHandle::try_new(page, *pos) else {
                continue;
            };
            if let Some((trans, handle)) = value {
                let handle = tile_set
                    .get_transformed_version(*trans, *handle)
                    .unwrap_or(*handle);
                self.insert(target_handle, TileDataUpdate::TransformSet(Some(handle)));
            } else {
                self.insert(target_handle, TileDataUpdate::TransformSet(None));
            }
        }
    }
    /// Get the color being set onto the given tile by this update, if a color is being set.
    pub fn get_color(&self, page: Vector2<i32>, position: Vector2<i32>) -> Option<Color> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        match self.get(&handle)? {
            TileDataUpdate::Erase => Some(Color::default()),
            TileDataUpdate::MaterialTile(data) => Some(data.color),
            TileDataUpdate::FreeformTile(def) => Some(def.data.color),
            TileDataUpdate::Color(color) => Some(*color),
            _ => None,
        }
    }
    /// Get the material being set onto the given tile by this update, if a material is being set.
    pub fn get_material(
        &self,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<MaterialUpdate> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        match self.get(&handle)? {
            TileDataUpdate::Erase => Some(MaterialUpdate::Erase),
            TileDataUpdate::FreeformTile(def) => {
                Some(MaterialUpdate::Replace(def.material_bounds.clone()))
            }
            TileDataUpdate::Material(mat) => Some(MaterialUpdate::Replace(mat.clone())),
            _ => None,
        }
    }
    /// Get the tile bounds being set onto the given tile by this update, if possible.
    pub fn get_tile_bounds(
        &self,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<TileBounds> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        match self.get(&handle)? {
            TileDataUpdate::Erase => Some(TileBounds::default()),
            TileDataUpdate::FreeformTile(def) => Some(def.material_bounds.bounds.clone()),
            TileDataUpdate::Material(mat) => Some(mat.bounds.clone()),
            _ => None,
        }
    }
    /// Get the value of the given property being set onto the given tile by this update, if possible.
    pub fn get_property(
        &self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        property_id: Uuid,
    ) -> Option<Option<TileSetPropertyValue>> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        match self.get(&handle)? {
            TileDataUpdate::Erase => Some(None),
            TileDataUpdate::MaterialTile(data) => Some(data.properties.get(&property_id).cloned()),
            TileDataUpdate::FreeformTile(def) => {
                Some(def.data.properties.get(&property_id).cloned())
            }
            TileDataUpdate::Property(id, value) if *id == property_id => Some(value.clone()),
            _ => None,
        }
    }
    /// Get the value of the given collider being set onto the given tile by this update, if possible.
    pub fn get_collider(
        &self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        collider_id: Uuid,
    ) -> Option<Option<TileCollider>> {
        let handle = TileDefinitionHandle::try_new(page, position)?;
        match self.get(&handle)? {
            TileDataUpdate::Erase => Some(None),
            TileDataUpdate::MaterialTile(data) => Some(data.collider.get(&collider_id).copied()),
            TileDataUpdate::FreeformTile(def) => Some(def.data.collider.get(&collider_id).copied()),
            TileDataUpdate::Collider(id, value) if *id == collider_id => Some(*value),
            _ => None,
        }
    }
    /// Set the given color on the given tile.
    pub fn set_color(&mut self, page: Vector2<i32>, position: Vector2<i32>, color: Color) {
        if let Some(handle) = TileDefinitionHandle::try_new(page, position) {
            self.insert(handle, TileDataUpdate::Color(color));
        }
    }
    /// Set the given property value on the given tile.
    pub fn set_property(
        &mut self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        property_id: Uuid,
        value: Option<TileSetPropertyValue>,
    ) {
        if let Some(handle) = TileDefinitionHandle::try_new(page, position) {
            self.insert(handle, TileDataUpdate::Property(property_id, value));
        }
    }
    /// Set the given value to the given slice of the given property of the given tile.
    pub fn set_property_slice(
        &mut self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        subposition: Vector2<usize>,
        property_id: Uuid,
        value: i8,
    ) {
        use TileSetPropertyValue as PropValue;
        let index = TileSetPropertyValue::nine_position_to_index(subposition);
        if let Some(handle) = TileDefinitionHandle::try_new(page, position) {
            match self.entry(handle) {
                Entry::Occupied(mut e) => match e.get_mut() {
                    TileDataUpdate::PropertySlice(uuid, d0) if *uuid == property_id => {
                        d0[index] = Some(value);
                    }
                    TileDataUpdate::Property(uuid, Some(PropValue::NineSlice(d0)))
                        if *uuid == property_id =>
                    {
                        d0[index] = value;
                    }
                    d0 => {
                        let mut data = [0; 9];
                        data[index] = value;
                        *d0 =
                            TileDataUpdate::Property(property_id, Some(PropValue::NineSlice(data)));
                    }
                },
                Entry::Vacant(e) => {
                    let mut data = [None; 9];
                    data[index] = Some(value);
                    let _ = e.insert(TileDataUpdate::PropertySlice(property_id, data));
                }
            }
        }
    }
    /// Set the given property value on the givne tile.
    pub fn set_collider(
        &mut self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        property_id: Uuid,
        value: TileCollider,
    ) {
        let value = match value {
            TileCollider::None => None,
            x => Some(x),
        };
        if let Some(handle) = TileDefinitionHandle::try_new(page, position) {
            self.insert(handle, TileDataUpdate::Collider(property_id, value));
        }
    }
    /// Set the given material on the given tile.
    pub fn set_material(
        &mut self,
        page: Vector2<i32>,
        position: Vector2<i32>,
        value: TileMaterialBounds,
    ) {
        if let Some(handle) = TileDefinitionHandle::try_new(page, position) {
            self.insert(handle, TileDataUpdate::Material(value));
        }
    }
}

type RotTileHandle = (OrthoTransformation, TileDefinitionHandle);

/// This is a step in the process of performing an edit to a tile map, brush, or tile set.
/// It provides handles for the tiles to be written and the transformation to apply to those
/// tiles.
#[derive(Clone, Debug, Default)]
pub struct TransTilesUpdate(TileGridMap<Option<RotTileHandle>>);

/// A set of changes to a set of tiles. A value of None indicates that a tile
/// is being removed from the set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TilesUpdate(TileGridMap<Option<TileDefinitionHandle>>);

impl Deref for TilesUpdate {
    type Target = TileGridMap<Option<TileDefinitionHandle>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TilesUpdate {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for TransTilesUpdate {
    type Target = TileGridMap<Option<RotTileHandle>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TransTilesUpdate {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TransTilesUpdate {
    /// Construct a TilesUpdate by finding the transformed version of each tile
    /// in the given tile set.
    pub fn build_tiles_update(&self, tile_set: &TileSet) -> TilesUpdate {
        let mut result = TilesUpdate::default();
        for (pos, value) in self.iter() {
            if let Some((trans, handle)) = value {
                result.insert(
                    *pos,
                    Some(
                        tile_set
                            .get_transformed_version(*trans, *handle)
                            .unwrap_or(*handle),
                    ),
                );
            } else {
                result.insert(*pos, None);
            }
        }
        result
    }
    /// Fills the given tiles at the given point using tiles from the given source. This method
    /// extends tile map when trying to fill at a point that lies outside the bounding rectangle.
    /// Keep in mind, that flood fill is only possible either on free cells or on cells with the same
    /// tile kind. Modifications to the tile source are written into the given TileUpdates object
    /// rather than modifying the tiles directly.
    pub fn flood_fill<S: TileSource>(
        &mut self,
        tiles: &Tiles,
        start_point: Vector2<i32>,
        brush: &S,
    ) {
        let mut bounds = tiles.bounding_rect();
        bounds.push(start_point);

        let allowed_definition = tiles.get_at(start_point);
        let mut stack = vec![start_point];
        while let Some(position) = stack.pop() {
            let definition = tiles.get_at(position);
            if definition == allowed_definition && !self.contains_key(&position) {
                let value = brush
                    .get_at(position - start_point)
                    .map(|h| (brush.transformation(), h));
                self.insert(position, value);

                // Continue on neighbours.
                for neighbour_position in [
                    Vector2::new(position.x - 1, position.y),
                    Vector2::new(position.x + 1, position.y),
                    Vector2::new(position.x, position.y - 1),
                    Vector2::new(position.x, position.y + 1),
                ] {
                    if bounds.contains(neighbour_position) {
                        stack.push(neighbour_position);
                    }
                }
            }
        }
    }
    /// Draws the given tiles on the tile map
    #[inline]
    pub fn draw_tiles(&mut self, origin: Vector2<i32>, brush: &Stamp) {
        let trans = brush.transformation();
        for (local_position, handle) in brush.iter() {
            self.insert(origin + local_position, Some((trans, *handle)));
        }
    }
    /// Erases the tiles under the given brush.
    #[inline]
    pub fn erase_stamp(&mut self, origin: Vector2<i32>, brush: &Stamp) {
        for local_position in brush.keys() {
            self.insert(origin + local_position, None);
        }
    }
    /// Erases the given tile.
    pub fn erase(&mut self, position: Vector2<i32>) {
        self.insert(position, None);
    }
    /// Fills the given rectangle using the given stamp.
    pub fn rect_fill(&mut self, start: Vector2<i32>, end: Vector2<i32>, stamp: &Stamp) {
        let region = TileRegion::from_points(start, end);
        let stamp_source = stamp.repeat(start, end);
        self.rect_fill_inner(region, &stamp_source);
    }
    /// Fills the given rectangle using random tiles from the given stamp.
    pub fn rect_fill_random(&mut self, start: Vector2<i32>, end: Vector2<i32>, stamp: &Stamp) {
        let region = TileRegion::from_points(start, end);
        self.rect_fill_inner(region, &RandomTileSource(stamp));
    }
    /// Fills the given rectangle using the given tiles.
    fn rect_fill_inner<S: TileSource>(&mut self, region: TileRegion, brush: &S) {
        let trans = brush.transformation();
        for (target, source) in region.iter() {
            if let Some(definition_handle) = brush.get_at(source) {
                self.insert(target, Some((trans, definition_handle)));
            }
        }
    }
    /// Draw a line from a point to point.
    pub fn draw_line<S: TileSource>(&mut self, from: Vector2<i32>, to: Vector2<i32>, brush: &S) {
        let trans = brush.transformation();
        for position in BresenhamLineIter::new(from, to) {
            if let Some(random_tile) = brush.get_at(position - from) {
                self.insert(position, Some((trans, random_tile)));
            }
        }
    }

    /// Fills in a rectangle using special brush with 3x3 tiles. It puts
    /// corner tiles in the respective corners of the target rectangle and draws lines between each
    /// corner using middle tiles.
    pub fn nine_slice(&mut self, start: Vector2<i32>, end: Vector2<i32>, brush: &Stamp) {
        self.nine_slice_inner(
            start,
            end,
            brush,
            |update, target_region, source, source_region| {
                update.rect_fill_inner(
                    target_region,
                    &RepeatTileSource {
                        source,
                        region: source_region,
                    },
                )
            },
        );
    }
    /// Fills in a rectangle using special brush with 3x3 tiles. It puts
    /// corner tiles in the respective corners of the target rectangle and draws lines between each
    /// corner using middle tiles shuffled into random order.
    pub fn nine_slice_random(&mut self, start: Vector2<i32>, end: Vector2<i32>, brush: &Stamp) {
        self.nine_slice_inner(
            start,
            end,
            brush,
            |update, target_region, source, source_region| {
                update.rect_fill_inner(
                    target_region,
                    &PartialRandomTileSource(source, source_region.bounds),
                )
            },
        );
    }

    /// Fills in a rectangle using special brush with 3x3 tiles. It puts
    /// corner tiles in the respective corners of the target rectangle and draws lines between each
    /// corner using middle tiles.
    #[inline]
    fn nine_slice_inner<F>(
        &mut self,
        start: Vector2<i32>,
        end: Vector2<i32>,
        stamp: &Stamp,
        fill: F,
    ) where
        F: Fn(&mut TransTilesUpdate, TileRegion, &Stamp, TileRegion),
    {
        let Some(stamp_rect) = *stamp.bounding_rect() else {
            return;
        };
        let rect = TileRect::from_points(start, end);
        let region = TileRegion {
            origin: start,
            bounds: rect.into(),
        };
        let inner_region = region.clone().deflate(1, 1);

        let stamp_region = TileRegion::from_bounds_and_direction(stamp_rect.into(), start - end);
        let mut inner_stamp_region = stamp_region.clone().deflate(1, 1);

        // Place corners first.
        let trans = stamp.transformation();
        for (corner_position, actual_corner_position) in [
            (stamp_rect.left_top_corner(), rect.left_top_corner()),
            (stamp_rect.right_top_corner(), rect.right_top_corner()),
            (stamp_rect.right_bottom_corner(), rect.right_bottom_corner()),
            (stamp_rect.left_bottom_corner(), rect.left_bottom_corner()),
        ] {
            if let Some(tile) = stamp.get(corner_position) {
                self.insert(actual_corner_position, Some((trans, *tile)));
            }
        }

        let top = region.clone().with_bounds(
            TileRect::from_points(
                rect.left_top_corner() + Vector2::new(1, 0),
                rect.right_top_corner() + Vector2::new(-1, 0),
            )
            .into(),
        );
        let bottom = region.clone().with_bounds(
            TileRect::from_points(
                rect.left_bottom_corner() + Vector2::new(1, 0),
                rect.right_bottom_corner() + Vector2::new(-1, 0),
            )
            .into(),
        );
        let left = region.clone().with_bounds(
            TileRect::from_points(
                rect.left_bottom_corner() + Vector2::new(0, 1),
                rect.left_top_corner() + Vector2::new(0, -1),
            )
            .into(),
        );
        let right = region.clone().with_bounds(
            TileRect::from_points(
                rect.right_bottom_corner() + Vector2::new(0, 1),
                rect.right_top_corner() + Vector2::new(0, -1),
            )
            .into(),
        );
        let stamp_top = stamp_region.clone().with_bounds(
            TileRect::from_points(
                stamp_rect.left_top_corner() + Vector2::new(1, 0),
                stamp_rect.right_top_corner() + Vector2::new(-1, 0),
            )
            .into(),
        );
        let stamp_bottom = stamp_region.clone().with_bounds(
            TileRect::from_points(
                stamp_rect.left_bottom_corner() + Vector2::new(1, 0),
                stamp_rect.right_bottom_corner() + Vector2::new(-1, 0),
            )
            .into(),
        );
        let stamp_left = stamp_region.clone().with_bounds(
            TileRect::from_points(
                stamp_rect.left_bottom_corner() + Vector2::new(0, 1),
                stamp_rect.left_top_corner() + Vector2::new(0, -1),
            )
            .into(),
        );
        let stamp_right = stamp_region.clone().with_bounds(
            TileRect::from_points(
                stamp_rect.right_bottom_corner() + Vector2::new(0, 1),
                stamp_rect.right_top_corner() + Vector2::new(0, -1),
            )
            .into(),
        );

        if rect.size.x > 2 && stamp_rect.size.x > 2 {
            fill(self, top, stamp, stamp_top);
            fill(self, bottom, stamp, stamp_bottom);
        }
        if rect.size.y > 2 && stamp_rect.size.y > 2 {
            fill(self, left, stamp, stamp_left);
            fill(self, right, stamp, stamp_right);
        }
        fill(self, inner_region, stamp, inner_stamp_region);
    }
}
