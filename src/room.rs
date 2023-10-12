use crate::device;

#[derive(Debug, Clone, PartialEq)]
pub enum RoomAction {
    Join,
    Leave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoomEvent {
    id: device::DeviceId,
    action: RoomAction,
}

impl RoomEvent {
    pub fn new(id: device::DeviceId, action: RoomAction) -> Self {
        RoomEvent { id, action }
    }

    pub fn id(&self) -> device::DeviceId {
        self.id
    }

    pub fn action(&self) -> &RoomAction {
        &self.action
    }

    pub fn set_id(&mut self, id: device::DeviceId) {
        self.id = id;
    }

    pub fn set_action(&mut self, action: RoomAction) {
        self.action = action;
    }
}
