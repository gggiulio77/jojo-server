use enigo::*;
use serde::{Deserialize, Serialize};

pub trait MouseDriver {
    fn mouse_move_relative(&mut self, x: i32, y: i32);
    fn mouse_move_down(&mut self, button: MouseButton);
    fn mouse_move_up(&mut self, button: MouseButton);
}

impl MouseDriver for Enigo {
    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        MouseControllable::mouse_move_relative(self, x, y)
    }
    fn mouse_move_down(&mut self, button: MouseButton) {
        MouseControllable::mouse_down(self, button)
    }
    fn mouse_move_up(&mut self, button: MouseButton) {
        MouseControllable::mouse_up(self, button)
    }
}

pub enum ButtonState {
    Hold,
    Idle,
}

// TODO: find a way to derive Copy/Clone
pub struct Mouse {
    x_sen: u8,
    y_sen: u8,
    click_left: ButtonState,
    driver: Box<dyn MouseDriver + Send>,
}

impl Default for Mouse {
    fn default() -> Mouse {
        Mouse {
            x_sen: 1,
            y_sen: 1,
            click_left: ButtonState::Idle,
            driver: Box::new(Enigo::new()),
        }
    }
}

impl Mouse {
    pub fn new(
        x_sen: u8,
        y_sen: u8,
        // TODO: think about how to store all buttons states
        click_left: ButtonState,
        driver: Box<dyn MouseDriver + Send>,
    ) -> Mouse {
        Mouse {
            x_sen,
            y_sen,
            click_left,
            driver,
        }
    }

    pub fn sensibility(&self) -> (u8, u8) {
        (self.x_sen, self.y_sen)
    }

    pub fn mouse_move_relative(&mut self, x: i32, y: i32) {
        self.driver.mouse_move_relative(x, y)
    }
    // TODO: generalize for all buttons
    pub fn mouse_move_down(&mut self, button: MouseButton) {
        match self.click_left {
            ButtonState::Idle => {
                self.driver.mouse_move_down(button);
                self.click_left = ButtonState::Hold;
            }
            ButtonState::Hold => {}
        }
    }
    // TODO: generalize for all buttons
    pub fn mouse_move_up(&mut self, button: MouseButton) {
        match self.click_left {
            ButtonState::Idle => {}
            ButtonState::Hold => {
                self.driver.mouse_move_up(button);
                self.click_left = ButtonState::Idle;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MouseRead {
    x_read: i32,
    y_read: i32,
    // TODO: generalize for all buttons
    click_read: bool,
}

impl MouseRead {
    pub fn new(x_read: i32, y_read: i32, click_read: bool) -> Self {
        MouseRead {
            x_read,
            y_read,
            click_read,
        }
    }

    pub fn reads(&self) -> (i32, i32, bool) {
        (self.x_read, self.y_read, self.click_read)
    }
}
