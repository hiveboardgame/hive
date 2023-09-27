use leptos_use::{
    breakpoints_master_css, use_breakpoints, BreakpointsMasterCss, UseBreakpointsReturn,
};

#[derive(Clone)]
pub struct ScreenSize {
    pub screensize: UseBreakpointsReturn<BreakpointsMasterCss>,
}

impl ScreenSize {
    pub fn new() -> Self {
        let breakpoints = breakpoints_master_css();
        Self {
            screensize: use_breakpoints(breakpoints.clone()),
        }
    }
}
