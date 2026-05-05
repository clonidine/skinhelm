#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapeRenderState {
    loaded: bool,
    visible: bool,
}

impl CapeRenderState {
    pub const fn new() -> Self {
        Self {
            loaded: false,
            visible: false,
        }
    }

    pub fn mark_loaded(&mut self) {
        self.loaded = true;
        self.visible = true;
    }

    pub fn clear(&mut self) {
        self.loaded = false;
        self.visible = false;
    }

    pub fn set_visible(&mut self, visible: bool) -> &'static str {
        if !self.loaded {
            self.visible = false;
            return "No cape loaded";
        }

        self.visible = visible;
        if visible {
            "Cape enabled"
        } else {
            "Cape disabled"
        }
    }

    pub fn should_render(self) -> bool {
        self.loaded && self.visible
    }
}

impl Default for CapeRenderState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_loaded_cape_does_not_render() {
        let state = CapeRenderState::new();

        assert!(!state.should_render());
    }

    #[test]
    fn visible_toggle_without_loaded_cape_stays_hidden() {
        let mut state = CapeRenderState::new();

        assert_eq!(state.set_visible(true), "No cape loaded");
        assert!(!state.should_render());
    }

    #[test]
    fn loaded_cape_renders_until_disabled() {
        let mut state = CapeRenderState::new();
        state.mark_loaded();

        assert!(state.should_render());
        assert_eq!(state.set_visible(false), "Cape disabled");
        assert!(!state.should_render());
    }
}
