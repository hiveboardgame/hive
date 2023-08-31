use bitfield_struct::bitfield;

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct GatedDirections {
    #[bits(1)]
    pub cached_w: bool,
    #[bits(1)]
    pub gated_w: bool,
    #[bits(1)]
    pub cached_nw: bool,
    #[bits(1)]
    pub gated_nw: bool,
    #[bits(1)]
    pub cached_ne: bool,
    #[bits(1)]
    pub gated_ne: bool,
    /// we need to fill the u8
    #[bits(2)]
    _padding: usize,
}
