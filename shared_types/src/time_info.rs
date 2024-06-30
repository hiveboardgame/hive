use crate::TimeMode;

#[derive(Clone, PartialEq)]
pub struct TimeInfo {
    pub mode: TimeMode,
    pub base: Option<i32>,
    pub increment: Option<i32>,
}
