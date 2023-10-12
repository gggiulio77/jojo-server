use enigo::{Enigo, MouseControllable};
use serde::{Deserialize, Serialize};

type EnigoMouseButton = enigo::MouseButton;

pub trait MouseAdapter {
    fn mouse_move_relative(&mut self, x: i32, y: i32);
    fn mouse_move_down(&mut self, button: EnigoMouseButton);
    fn mouse_move_up(&mut self, button: EnigoMouseButton);
}

impl MouseAdapter for Enigo {
    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        println!("moving mouse {x} horizontal and {y} vertical");
        MouseControllable::mouse_move_relative(self, x, y)
    }
    fn mouse_move_down(&mut self, button: EnigoMouseButton) {
        MouseControllable::mouse_down(self, button)
    }
    fn mouse_move_up(&mut self, button: EnigoMouseButton) {
        MouseControllable::mouse_up(self, button)
    }
}

// TODO: find a way to derive Clone, Enigo does not impl CLONE (mayor fuck)
pub struct MouseDriver(Box<dyn MouseAdapter + Send>);

impl Default for MouseDriver {
    fn default() -> MouseDriver {
        MouseDriver(Box::new(Enigo::new()))
    }
}

impl MouseDriver {
    pub fn new(driver: Box<dyn MouseAdapter + Send>) -> MouseDriver {
        MouseDriver(driver)
    }

    pub fn mouse_move_relative(&mut self, x: i32, y: i32) {
        self.0.mouse_move_relative(x, y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct MouseRead {
    x_read: i32,
    y_read: i32,
}

impl MouseRead {
    pub fn new(x_read: i32, y_read: i32) -> Self {
        MouseRead { x_read, y_read }
    }
    pub fn x_read(&self) -> i32 {
        self.x_read
    }
    pub fn y_read(&self) -> i32 {
        self.y_read
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct MouseConfig {
    x_sen: i8,
    y_sen: i8,
}

impl Default for MouseConfig {
    fn default() -> Self {
        MouseConfig {
            x_sen: 1,
            y_sen: -1,
        }
    }
}

impl MouseConfig {
    fn new(x_sen: i8, y_sen: i8) -> Self {
        MouseConfig { x_sen, y_sen }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_pointer() {
        let mut mouse_driver = MouseDriver::default();

        mouse_driver.mouse_move_relative(50, 50);
    }
}
