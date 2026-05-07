use enigo::*;
fn main() {
    let mut enigo = Enigo::new();
    enigo.mouse_move_to(0, 0);
    enigo.mouse_click(MouseButton::Left);
}