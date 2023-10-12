use enigo::Enigo;

type EnigoKey = enigo::Key;

pub trait KeyboardAdapter {
    fn key_sequence(&mut self, sequence: &str);
    fn key_click(&mut self, key: EnigoKey);
    fn key_down(&mut self, key: EnigoKey);
    fn key_up(&mut self, key: EnigoKey);
}

// TODO: implement this adaptation layer
// TODO: study each fn works in Enigo
impl KeyboardAdapter for Enigo {
    fn key_sequence(&mut self, sequence: &str) {
        todo!()
    }
    fn key_click(&mut self, key: EnigoKey) {
        todo!()
    }
    fn key_down(&mut self, key: EnigoKey) {
        todo!()
    }
    fn key_up(&mut self, key: EnigoKey) {
        todo!()
    }
}
