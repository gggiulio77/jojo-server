use crate::driver;
use enigo::Enigo;

type EnigoMouseButton = enigo::MouseButton;

pub struct ButtonDriver {
    mouse_driver: Box<dyn driver::mouse::MouseAdapter + Send>,
    keyboard_driver: Box<dyn driver::keyboard::KeyboardAdapter + Send>,
}

impl Default for ButtonDriver {
    fn default() -> Self {
        ButtonDriver {
            mouse_driver: Box::new(Enigo::new()),
            keyboard_driver: Box::new(Enigo::new()),
        }
    }
}

impl<'a> ButtonDriver {
    pub fn new(
        mouse_driver: Box<dyn driver::mouse::MouseAdapter + Send>,
        keyboard_driver: Box<dyn driver::keyboard::KeyboardAdapter + Send>,
    ) -> ButtonDriver {
        ButtonDriver {
            mouse_driver,
            keyboard_driver,
        }
    }

    fn decode(
        &self,
        mouse_button: &'a jojo_common::mouse::MouseButton,
    ) -> (EnigoMouseButton, &'a jojo_common::mouse::MouseButtonState) {
        return match mouse_button {
            jojo_common::mouse::MouseButton::Left(state) => (EnigoMouseButton::Left, state),
            jojo_common::mouse::MouseButton::Right(state) => (EnigoMouseButton::Right, state),
            jojo_common::mouse::MouseButton::Back(state) => (EnigoMouseButton::Back, state),
            jojo_common::mouse::MouseButton::Forward(state) => (EnigoMouseButton::Forward, state),
            jojo_common::mouse::MouseButton::Middle(state) => (EnigoMouseButton::Middle, state),
            jojo_common::mouse::MouseButton::ScrollDown(state) => {
                (EnigoMouseButton::ScrollDown, state)
            }
            jojo_common::mouse::MouseButton::ScrollLeft(state) => {
                (EnigoMouseButton::ScrollLeft, state)
            }
            jojo_common::mouse::MouseButton::ScrollRight(state) => {
                (EnigoMouseButton::ScrollRight, state)
            }
            jojo_common::mouse::MouseButton::ScrollUp(state) => (EnigoMouseButton::ScrollUp, state),
        };
    }

    // TODO: think a better way to do this, maybe a impl into o something similar
    pub fn button_to_state(&mut self, mouse_button: &jojo_common::mouse::MouseButton) {
        let (button, state) = self.decode(mouse_button);
        match state {
            jojo_common::mouse::MouseButtonState::Up => self.mouse_driver.mouse_move_up(button),
            jojo_common::mouse::MouseButtonState::Down => self.mouse_driver.mouse_move_down(button),
        }
    }
}
