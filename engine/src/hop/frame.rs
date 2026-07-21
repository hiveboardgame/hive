use crate::position::Rotation;

/// Pepeke's hex frame mirrors the engine's, so HOP's `+` is a counter-clockwise step here.
pub(super) const HOP_PLUS: Rotation = Rotation::CC;
pub(super) const HOP_MINUS: Rotation = Rotation::C;
