// Test
use super::*;

#[derive(Copy, Clone)]
struct Foo(f32);

impl MetricSpace for Foo {
    type Distance = f32;
    type UserData = ();
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        (self.0 - other.0).abs()
    }
}

#[derive(Copy, Clone)]
struct Bar(i32);

impl MetricSpace for Bar {
    type UserData = usize;
    type Distance = u32;

    fn distance(&self, other: &Self, user_data: &Self::UserData) -> Self::Distance {
        assert_eq!(12345, *user_data);

        (self.0 - other.0).abs() as u32
    }
}

#[test]
fn test_without_user_data() {
    let foos = [Foo(1.0), Foo(1.5), Foo(2.0)];
    let vp = Tree::new(&foos);

    assert_eq!((2, 98.0), vp.find_nearest(&Foo(100.0)));
    assert_eq!((0, 101.0), vp.find_nearest(&Foo(-100.0)));
    assert_eq!((1, 0.0), vp.find_nearest(&Foo(1.5)));
    assert_eq!((1, 0.125), vp.find_nearest(&Foo(1.5-0.125)));
    assert_eq!((2, 0.125), vp.find_nearest(&Foo(2.0-0.125)));
}

#[test]
fn test_with_user_data() {
    let bars = [Bar(10), Bar(15), Bar(20)];
    let magic = 12345;
    let vp = Tree::new_with_user_data_owned(&bars, magic);

    assert_eq!((1, 0), vp.find_nearest(&Bar(15)));
    assert_eq!((1, 1), vp.find_nearest_with_user_data(&Bar(16), &magic));

    let vp = Tree::new_with_user_data_ref(&bars, &magic);
    assert_eq!((0, 1), vp.find_nearest(&Bar(9), &magic));
    assert_eq!((0, 1), vp.find_nearest_with_user_data(&Bar(9), &magic));
}
