use enigo::*;
use serde::{Deserialize, Serialize};

pub trait MouseDriver {
    fn mouse_move_relative(&mut self, x: i32, y: i32);
}

impl MouseDriver for Enigo {
    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        MouseControllable::mouse_move_relative(self, x, y)
    }
}

// TODO: find a way to derive Copy/Clone
pub struct Mouse {
    pub x_sen: u8,
    pub y_sen: u8,
    driver: Box<dyn MouseDriver + Send>,
}

impl Default for Mouse {
    fn default() -> Mouse {
        Mouse {
            x_sen: 1,
            y_sen: 1,
            driver: Box::new(Enigo::new()),
        }
    }
}

impl Mouse {
    pub fn new(x_sen: u8, y_sen: u8, driver: Box<dyn MouseDriver + Send>) -> Mouse {
        Mouse {
            x_sen,
            y_sen,
            driver,
        }
    }

    pub fn mouse_move_relative(&mut self, x: i32, y: i32) {
        self.driver.mouse_move_relative(x, y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MouseRead {
    pub x_read: i32,
    pub y_read: i32,
}
