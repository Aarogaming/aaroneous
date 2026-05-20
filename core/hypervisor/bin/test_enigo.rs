use enigo::*;
fn main() {
    let mut enigo = Enigo::new(&enigo::Settings::default()).unwrap();
    let _ = enigo.move_mouse(0, 0, Coordinate::Abs);
    let _ = enigo.button(Button::Left, Direction::Click);
}