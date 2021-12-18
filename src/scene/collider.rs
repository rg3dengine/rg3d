#![allow(missing_docs)]

use crate::{
    core::{
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::geometry::{ColliderHandle, InteractionGroups},
    scene::{
        base::{Base, BaseBuilder},
        graph::{
            physics::{ContactPair, PhysicsWorld},
            Graph,
        },
        node::Node,
    },
};
use bitflags::bitflags;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

bitflags! {
    pub(crate) struct ColliderChanges: u32 {
        const NONE = 0;
        const SHAPE = 0b0000_0001;
        const RESTITUTION = 0b0000_0010;
        const COLLISION_GROUPS = 0b0000_0100;
        const FRICTION = 0b0000_1000;
        const FRICTION_COMBINE_RULE = 0b0001_0000;
        const RESTITUTION_COMBINE_RULE = 0b0010_0000;
        const IS_SENSOR = 0b0100_0000;
        const SOLVER_GROUPS = 0b1000_0000;
        const DENSITY = 0b0001_0000_0000;
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct BallShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for BallShape {
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct CylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for CylinderShape {
    fn default() -> Self {
        Self {
            half_height: 0.5,
            radius: 0.5,
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct RoundCylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub border_radius: f32,
}

impl Default for RoundCylinderShape {
    fn default() -> Self {
        Self {
            half_height: 0.5,
            radius: 0.5,
            border_radius: 0.1,
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct ConeShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for ConeShape {
    fn default() -> Self {
        Self {
            half_height: 0.5,
            radius: 0.5,
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct CuboidShape {
    pub half_extents: Vector3<f32>,
}

impl Default for CuboidShape {
    fn default() -> Self {
        Self {
            half_extents: Vector3::new(0.5, 0.5, 0.5),
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct CapsuleShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

impl Default for CapsuleShape {
    // Y-capsule
    fn default() -> Self {
        Self {
            begin: Default::default(),
            end: Vector3::new(0.0, 1.0, 0.0),
            radius: 0.5,
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct SegmentShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
}

impl Default for SegmentShape {
    fn default() -> Self {
        Self {
            begin: Default::default(),
            end: Vector3::new(0.0, 1.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Visit, Inspect)]
pub struct TriangleShape {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
}

impl Default for TriangleShape {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Vector3::new(1.0, 0.0, 0.0),
            c: Vector3::new(0.0, 0.0, 1.0),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug, Visit, Inspect)]
pub struct GeometrySource(pub Handle<Node>);

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TrimeshShape {
    pub sources: Vec<GeometrySource>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct HeightfieldShape {
    pub geometry_source: GeometrySource,
}

#[doc(hidden)]
#[derive(Visit, Debug, Clone, Copy, Inspect)]
pub struct InteractionGroupsDesc {
    pub memberships: u32,
    pub filter: u32,
}

impl InteractionGroupsDesc {
    pub fn new(memberships: u32, filter: u32) -> Self {
        Self {
            memberships,
            filter,
        }
    }
}

impl Default for InteractionGroupsDesc {
    fn default() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

impl From<InteractionGroups> for InteractionGroupsDesc {
    fn from(g: InteractionGroups) -> Self {
        Self {
            memberships: g.memberships,
            filter: g.filter,
        }
    }
}

impl Inspect for ColliderShape {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            ColliderShape::Ball(v) => v.properties(),
            ColliderShape::Cylinder(v) => v.properties(),
            ColliderShape::RoundCylinder(v) => v.properties(),
            ColliderShape::Cone(v) => v.properties(),
            ColliderShape::Cuboid(v) => v.properties(),
            ColliderShape::Capsule(v) => v.properties(),
            ColliderShape::Segment(v) => v.properties(),
            ColliderShape::Triangle(v) => v.properties(),
            ColliderShape::Trimesh(v) => v.properties(),
            ColliderShape::Heightfield(v) => v.properties(),
        }
    }
}

#[derive(Clone, Debug, Visit)]
pub enum ColliderShape {
    Ball(BallShape),
    Cylinder(CylinderShape),
    RoundCylinder(RoundCylinderShape),
    Cone(ConeShape),
    Cuboid(CuboidShape),
    Capsule(CapsuleShape),
    Segment(SegmentShape),
    Triangle(TriangleShape),
    Trimesh(TrimeshShape),
    Heightfield(HeightfieldShape),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShape {
    /// Initializes a ball shape defined by its radius.
    pub fn ball(radius: f32) -> Self {
        Self::Ball(BallShape { radius })
    }

    /// Initializes a cylindrical shape defined by its half-height (along along the y axis) and its
    /// radius.
    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self::Cylinder(CylinderShape {
            half_height,
            radius,
        })
    }

    /// Initializes a rounded cylindrical shape defined by its half-height (along along the y axis),
    /// its radius, and its roundness (the radius of the sphere used for dilating the cylinder).
    pub fn round_cylinder(half_height: f32, radius: f32, border_radius: f32) -> Self {
        Self::RoundCylinder(RoundCylinderShape {
            half_height,
            radius,
            border_radius,
        })
    }

    /// Initializes a cone shape defined by its half-height (along along the y axis) and its basis
    /// radius.
    pub fn cone(half_height: f32, radius: f32) -> Self {
        Self::Cone(ConeShape {
            half_height,
            radius,
        })
    }

    /// Initializes a cuboid shape defined by its half-extents.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self::Cuboid(CuboidShape {
            half_extents: Vector3::new(hx, hy, hz),
        })
    }

    /// Initializes a capsule shape from its endpoints and radius.
    pub fn capsule(begin: Vector3<f32>, end: Vector3<f32>, radius: f32) -> Self {
        Self::Capsule(CapsuleShape { begin, end, radius })
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `x` axis.
    pub fn capsule_x(half_height: f32, radius: f32) -> Self {
        let p = Vector3::x() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `y` axis.
    pub fn capsule_y(half_height: f32, radius: f32) -> Self {
        let p = Vector3::y() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `z` axis.
    pub fn capsule_z(half_height: f32, radius: f32) -> Self {
        let p = Vector3::z() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a segment shape from its endpoints.
    pub fn segment(begin: Vector3<f32>, end: Vector3<f32>) -> Self {
        Self::Segment(SegmentShape { begin, end })
    }

    /// Initializes a triangle shape.
    pub fn triangle(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> Self {
        Self::Triangle(TriangleShape { a, b, c })
    }

    /// Initializes a triangle mesh shape defined by a set of handles to mesh nodes that will be
    /// used to create physical shape.
    pub fn trimesh(geometry_sources: Vec<GeometrySource>) -> Self {
        Self::Trimesh(TrimeshShape {
            sources: geometry_sources,
        })
    }

    /// Initializes a heightfield shape defined by a handle to terrain node.
    pub fn heightfield(geometry_source: GeometrySource) -> Self {
        Self::Heightfield(HeightfieldShape { geometry_source })
    }
}

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    shape: ColliderShape,
    #[inspect(min_value = 0.0, step = 0.05)]
    friction: f32,
    density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: Cell<ColliderHandle>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) changes: Cell<ColliderChanges>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        }
    }
}

impl Deref for Collider {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Collider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct ColliderShapeRefMut<'a> {
    parent: &'a mut Collider,
}

impl<'a> Drop for ColliderShapeRefMut<'a> {
    fn drop(&mut self) {
        self.parent.changes.get_mut().insert(ColliderChanges::SHAPE);
    }
}

impl<'a> Deref for ColliderShapeRefMut<'a> {
    type Target = ColliderShape;

    fn deref(&self) -> &Self::Target {
        &self.parent.shape
    }
}

impl<'a> DerefMut for ColliderShapeRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent.shape
    }
}

impl Collider {
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            shape: self.shape.clone(),
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            // Do not copy.
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        }
    }

    pub fn set_shape(&mut self, shape: ColliderShape) {
        self.shape = shape;
        self.changes.get_mut().insert(ColliderChanges::SHAPE);
    }

    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    pub fn shape_value(&self) -> ColliderShape {
        self.shape.clone()
    }

    pub fn shape_mut(&mut self) -> ColliderShapeRefMut {
        ColliderShapeRefMut { parent: self }
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
        self.changes.get_mut().insert(ColliderChanges::RESTITUTION);
    }

    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    pub fn set_density(&mut self, density: Option<f32>) {
        self.density = density;
        self.changes.get_mut().insert(ColliderChanges::DENSITY);
    }

    pub fn density(&self) -> Option<f32> {
        self.density
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
        self.changes.get_mut().insert(ColliderChanges::FRICTION);
    }

    pub fn friction(&self) -> f32 {
        self.friction
    }

    pub fn set_collision_groups(&mut self, groups: InteractionGroupsDesc) {
        self.collision_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::COLLISION_GROUPS);
    }

    pub fn collision_groups(&self) -> InteractionGroupsDesc {
        self.collision_groups
    }

    pub fn set_solver_groups(&mut self, groups: InteractionGroupsDesc) {
        self.solver_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::SOLVER_GROUPS);
    }

    pub fn solver_groups(&self) -> InteractionGroupsDesc {
        self.solver_groups
    }

    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
        self.changes.get_mut().insert(ColliderChanges::IS_SENSOR);
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    pub fn contacts<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = ContactPair> + 'a {
        physics.contacts_with(self.native.get())
    }
}

pub struct ColliderBuilder {
    base_builder: BaseBuilder,
    shape: ColliderShape,
    friction: f32,
    density: Option<f32>,
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
}

impl ColliderBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }

    pub fn with_shape(mut self, shape: ColliderShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn build_node(self) -> Node {
        let collider = Collider {
            base: self.base_builder.build_base(),
            shape: self.shape,
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        };
        Node::Collider(collider)
    }

    pub fn with_density(mut self, density: Option<f32>) -> Self {
        self.density = density;
        self
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn with_sensor(mut self, sensor: bool) -> Self {
        self.is_sensor = sensor;
        self
    }

    pub fn with_solver_groups(mut self, solver_groups: InteractionGroupsDesc) -> Self {
        self.solver_groups = solver_groups;
        self
    }

    pub fn with_collision_groups(mut self, collision_groups: InteractionGroupsDesc) -> Self {
        self.collision_groups = collision_groups;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
