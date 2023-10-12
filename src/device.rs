use crate::{button, mouse};
use serde::{Deserialize, Serialize};

pub type DeviceId = uuid::Uuid;

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct Device {
    id: DeviceId,
    name: String,
    mouse_config: Option<mouse::MouseConfig>,
    // TODO: maybe use a hash or static array
    buttons: Vec<button::Button>,
}

impl Device {
    pub fn new(
        id: DeviceId,
        name: String,
        mouse_config: Option<mouse::MouseConfig>,
        buttons: Vec<button::Button>,
    ) -> Self {
        Device {
            id,
            name,
            mouse_config,
            buttons,
        }
    }

    pub fn id(&self) -> DeviceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mouse_config(&self) -> &Option<mouse::MouseConfig> {
        &self.mouse_config
    }

    pub fn buttons(&self) -> &Vec<button::Button> {
        &self.buttons
    }

    pub fn set_id(&mut self, id: DeviceId) {
        self.id = id;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_mouse_config(&mut self, mouse_config: Option<mouse::MouseConfig>) {
        self.mouse_config = mouse_config;
    }

    pub fn set_buttons(&mut self, buttons: Vec<button::Button>) {
        self.buttons = buttons;
    }
}
