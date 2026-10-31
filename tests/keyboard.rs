use air01::keyboard::{self, Direction};
use ratatui::layout::Rect;
#[test]
fn complete_physical_layout_has_unique_slots_and_correct_special_positions() {
    let ks = keyboard::keys();
    assert_eq!(ks.len(), 84);
    let mut slots = ks.iter().map(|k| k.slot).collect::<Vec<_>>();
    slots.sort();
    slots.dedup();
    assert_eq!(slots.len(), 84);
    for (slot, legend, row) in [
        (5, "Fn", 5),
        (11, "Ctrl", 5),
        (17, "Opt", 5),
        (23, "Cmd", 5),
        (65, "Cmd", 5),
        (71, "Opt", 5),
        (82, "↑", 4),
        (87, "Light", 0),
    ] {
        let k = ks.iter().find(|k| k.slot == slot).unwrap();
        assert_eq!(k.legend, legend);
        assert_eq!(k.y, row);
    }
}
#[test]
fn scaled_keycaps_and_mouse_targets_share_geometry() {
    for (w, h, large) in [(78, 13, false), (118, 19, true), (158, 25, true)] {
        let a = Rect::new(1, 4, w, h);
        let rs = keyboard::rectangles(a, large);
        assert_eq!(rs.len(), 84);
        for (slot, r) in &rs {
            assert!(r.width >= 3 && r.height >= 3);
            assert!(r.x >= a.x && r.right() <= a.right() && r.bottom() <= a.bottom());
            assert_eq!(
                keyboard::hit(a, large, r.x + r.width / 2, r.y + 1),
                Some(*slot)
            );
        }
    }
}
#[test]
fn arrows_follow_physical_adjacency_not_matrix_order() {
    assert_eq!(keyboard::neighbor(5, Direction::Right), 11);
    assert_eq!(keyboard::neighbor(11, Direction::Left), 5);
    assert_eq!(keyboard::neighbor(5, Direction::Up), 4);
    assert_eq!(keyboard::neighbor(82, Direction::Down), 83);
    assert_eq!(keyboard::neighbor(83, Direction::Left), 77);
    assert_eq!(keyboard::neighbor(83, Direction::Right), 89);
    assert_eq!(keyboard::neighbor(87, Direction::Down), 85);
}
