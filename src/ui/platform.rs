use crate::ui::window_adapter::MinimalFemtoVGWindow;
use slint::{
    platform::{Platform, WindowAdapter},
    PlatformError,
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

pub struct CthulockSlintPlatform {
    window: Rc<MinimalFemtoVGWindow>,

    start_time: Instant,
}

impl CthulockSlintPlatform {
    pub fn new(window: Rc<MinimalFemtoVGWindow>) -> Self {
        Self {
            window,
            start_time: Instant::now(),
        }
    }
}

impl Platform for CthulockSlintPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.start_time.elapsed()
    }
}
