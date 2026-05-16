//! Schematic electrical connectivity (union-find, net naming).
//!
//! Shared by the native editor live graph and document export.

mod disjoint_set;
mod net_name;
mod point;
mod rebuild;

pub use disjoint_set::{DisjointSet, PointKey};
pub use net_name::sanitize_net_name;
pub use point::{point_key_f32, point_key_xy, POINT_QUANTUM};
pub use rebuild::{
    rebuild_connectivity, ConnLabel, ConnPin, ConnPoint, ConnPower, ConnSegment, ConnectivityInput,
    ConnectivityResult, LabelKind,
};
