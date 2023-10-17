use log::*;
use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

trait CustomMap<K, V>: dyn_clone::DynClone + Sync + Send {
    fn insert(&mut self, key: K, value: V) -> Option<V>;
    fn keys(&self) -> Keys<'_, K, V>;
    fn get(&self, key: &K) -> Option<&V>;

    fn remove(&mut self, key: &jojo_common::device::DeviceId) -> Option<V>;
}

dyn_clone::clone_trait_object!(<K, V> CustomMap<K, V>);

impl CustomMap<jojo_common::device::DeviceId, jojo_common::device::Device>
    for HashMap<jojo_common::device::DeviceId, jojo_common::device::Device>
{
    fn insert(
        &mut self,
        key: jojo_common::device::DeviceId,
        value: jojo_common::device::Device,
    ) -> Option<jojo_common::device::Device> {
        HashMap::insert(self, key, value)
    }

    fn keys(&self) -> Keys<'_, jojo_common::device::DeviceId, jojo_common::device::Device> {
        HashMap::keys(self)
    }

    fn get(&self, key: &jojo_common::device::DeviceId) -> Option<&jojo_common::device::Device> {
        HashMap::get(self, key)
    }

    fn remove(
        &mut self,
        key: &jojo_common::device::DeviceId,
    ) -> Option<jojo_common::device::Device> {
        HashMap::remove(self, key)
    }
}

#[derive(Clone)]
pub struct DeviceMap {
    custom_map: Box<dyn CustomMap<jojo_common::device::DeviceId, jojo_common::device::Device>>,
}

impl DeviceMap {
    pub fn new() -> Self {
        DeviceMap {
            custom_map: Box::new(HashMap::new()),
        }
    }

    pub fn insert(
        &mut self,
        key: jojo_common::device::DeviceId,
        value: jojo_common::device::Device,
        sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
    ) -> Option<jojo_common::device::Device> {
        info!("[DeviceMap]: insert key: {:?}, value: {:?}", key, value);

        sender
            .send(jojo_common::room::RoomEvent::new(
                key.clone(),
                jojo_common::room::RoomAction::Join,
            ))
            .unwrap();

        self.custom_map.insert(key, value)
    }

    pub fn keys(&self) -> Keys<'_, jojo_common::device::DeviceId, jojo_common::device::Device> {
        self.custom_map.keys()
    }

    pub fn get(&self, key: &jojo_common::device::DeviceId) -> Option<&jojo_common::device::Device> {
        self.custom_map.get(key)
    }

    pub fn remove(
        &mut self,
        key: &jojo_common::device::DeviceId,
        sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
    ) -> Option<jojo_common::device::Device> {
        info!("[DeviceMap]: remove key: {:?}", key);

        sender
            .send(jojo_common::room::RoomEvent::new(
                key.clone(),
                jojo_common::room::RoomAction::Leave,
            ))
            .unwrap();

        self.custom_map.remove(&key)
    }
}

pub type Devices = Arc<RwLock<DeviceMap>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_map() {
        let (tx, rx) = crossbeam_channel::unbounded::<jojo_common::room::RoomEvent>();
        let key = jojo_common::device::DeviceId::new_v4();
        let device = jojo_common::device::Device::default();

        let mut device_map = DeviceMap::new();

        device_map.insert(key, device, tx.clone());

        let result = device_map.get(&key).unwrap().clone();

        device_map.remove(&key, tx);

        let insert_event = rx.recv().unwrap();
        let remove_event = rx.recv().unwrap();

        assert_eq!(result, jojo_common::device::Device::default());
        assert_eq!(
            insert_event,
            jojo_common::room::RoomEvent::new(key, jojo_common::room::RoomAction::Join)
        );
        assert_eq!(
            remove_event,
            jojo_common::room::RoomEvent::new(key, jojo_common::room::RoomAction::Leave)
        );
    }

    #[tokio::test]
    async fn test_device_map_thread_safe() {
        let (tx, rx) = crossbeam_channel::unbounded::<jojo_common::room::RoomEvent>();
        let key = jojo_common::device::DeviceId::new_v4();
        let device = jojo_common::device::Device::default();

        let device_map_lock = Arc::new(RwLock::new(DeviceMap::new()));

        device_map_lock
            .write()
            .await
            .insert(key, device, tx.clone());

        let result = device_map_lock.read().await.get(&key).unwrap().clone();

        device_map_lock.write().await.remove(&key, tx);

        let insert_event = rx.recv().unwrap();
        let remove_event = rx.recv().unwrap();

        assert_eq!(result, jojo_common::device::Device::default());
        assert_eq!(
            insert_event,
            jojo_common::room::RoomEvent::new(key, jojo_common::room::RoomAction::Join)
        );
        assert_eq!(
            remove_event,
            jojo_common::room::RoomEvent::new(key, jojo_common::room::RoomAction::Leave)
        );
    }
}
