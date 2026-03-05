use enigo::{Enigo, Keyboard, Settings};

fn main() {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.text("Hello").unwrap();
}
