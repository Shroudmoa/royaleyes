//! Tests for the small state menu: navigation, selection, drawing.

use royaleyes::frame::Frame;
use royaleyes::menu::Menu;
use royaleyes::state::EyeState;

#[test]
fn menu_navigation_wraps_around() {
    let mut menu = Menu::new();
    menu.open(EyeState::Warning);
    assert!(menu.is_open());
    assert_eq!(menu.selected(), EyeState::Warning);

    menu.select_next();
    assert_eq!(menu.selected(), EyeState::Critical);

    menu.select_next();
    assert_eq!(menu.selected(), EyeState::Normal);

    menu.select_next();
    assert_eq!(menu.selected(), EyeState::Alert);

    menu.select_prev();
    assert_eq!(menu.selected(), EyeState::Normal);

    menu.close();
    assert!(!menu.is_open());
}

#[test]
fn menu_closed_has_no_selection() {
    let menu = Menu::new();
    assert!(!menu.is_open());
}

#[test]
fn menu_draw_renders_for_a_roomy_terminal() {
    let menu = Menu::new();
    let mut f = Frame::new(80, 24);
    assert!(menu.draw(&mut f, EyeState::Critical));
}

#[test]
fn menu_draw_skips_small_terminals() {
    let menu = Menu::new();
    let mut f = Frame::new(80, 3);
    assert!(!menu.draw(&mut f, EyeState::Normal));
}
