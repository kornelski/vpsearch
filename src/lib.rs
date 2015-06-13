use std::cmp::Ordering;
use std::ops::Add;
use std::ops::Sub;

pub trait MaximumValue {
    fn max() -> Self;
}

impl MaximumValue for f32 {
    fn max() -> Self {
        std::f32::MAX
    }
}

pub trait MetricSpace {
    type UserData = ();
    type Distance: Copy + PartialOrd + MaximumValue + Add<Output=<Self as MetricSpace>::Distance> + Sub<Output=<Self as MetricSpace>::Distance> = f32;

    /**
     * This function must return distance between two items that meets triangle inequality.
     * Specifically, it can't return squared distance (you must use sqrt if you use Euclidean distance)
     *
     * @param user_data Whatever you want. Passed from new_with_user_data()
     */
    fn distance(&self, other: &Self, user_data: &Self::UserData) -> Self::Distance;
}

struct Node<Item: MetricSpace + Copy> {
    near: Option<Box<Node<Item>>>,
    far: Option<Box<Node<Item>>>,
    vantage_point: Item, // Pointer to the item (value) represented by the current node
    radius: <Item as MetricSpace>::Distance,    // How far the `near` node stretches
    idx: usize,             // Index of the `vantage_point` in the original items array
}

pub struct Tree<'a, Item: MetricSpace + Copy> where <Item as MetricSpace>::UserData: 'a {
    root: Node<Item>,
    user_data: &'a <Item as MetricSpace>::UserData,
}

/* Temporary object used to reorder/track distance between items without modifying the orignial items array
   (also used during search to hold the two properties).
*/
struct Tmp<Item: MetricSpace> {
    distance: <Item as MetricSpace>::Distance,
    idx: usize,
}

static DUMMY_DATA: () = ();

impl<Item: MetricSpace<UserData = ()> + Copy> Tree<'static, Item> where <Item as MetricSpace>::UserData: 'static {

    /**
     * @sea Tree::new_with_user_data
     */
    pub fn new(items: &[Item]) -> Tree<'static, Item> {
        Self::new_with_user_data(items, &DUMMY_DATA)
    }
}

impl<'a, Item: MetricSpace + Copy> Tree<'a, Item> {
    fn sort_indexes_by_distance(vantage_point: Item, indexes: &mut [Tmp<Item>], items: &[Item], user_data: &<Item as MetricSpace>::UserData) {
        for i in indexes.iter_mut() {
            i.distance = vantage_point.distance(&items[i.idx], user_data);
        }
        indexes.sort_by(|a, b| if a.distance < b.distance {Ordering::Less} else {Ordering::Greater});
    }

    fn create_node(indexes: &mut [Tmp<Item>], items: &[Item], user_data: &<Item as MetricSpace>::UserData) -> Option<Node<Item>> {
        if indexes.len() == 0 {
            return None;
        }

        if indexes.len() == 1 {
            return Some(Node{
                near: None, far: None,
                vantage_point: items[indexes[0].idx],
                idx: indexes[0].idx,
                radius: <<Item as MetricSpace>::Distance as MaximumValue>::max(),
            });
        }

        let ref_idx = indexes[0].idx;

        // Removes the `ref_idx` item from remaining items, because it's included in the current node
        let rest = &mut indexes[1..];

        Self::sort_indexes_by_distance(items[ref_idx], rest, items, user_data);

        // Remaining items are split by the median distance
        let half_idx = rest.len()/2;

        let (near_indexes, far_indexes) = rest.split_at_mut(half_idx);

        Some(Node{
            vantage_point: items[ref_idx],
            idx: ref_idx,
            radius: far_indexes[0].distance,
            near: Self::create_node(near_indexes, items, user_data).map(|i| Box::new(i)),
            far: Self::create_node(far_indexes, items, user_data).map(|i| Box::new(i)),
        })
    }

    /**
     * Create a Vantage Point tree for fast nearest neighbor search.
     *
     * @param  items        Array of items that will be searched.
     * @param  user_data    Reference to any object that is passed down to item.distance()
     */
    pub fn new_with_user_data(items: &[Item], user_data: &'a <Item as MetricSpace>::UserData) -> Tree<'a, Item> {
        let mut indexes: Vec<_> = (0..items.len()).map(|i| Tmp{
            idx:i, distance: <<Item as MetricSpace>::Distance as MaximumValue>::max(),
        }).collect();

        Tree {
            root: Self::create_node(&mut indexes[..], items, user_data).unwrap(),
            user_data: user_data,
        }
    }

    fn search_node(node: &Node<Item>, needle: &Item, best_candidate: &mut Tmp<Item>, user_data: &<Item as MetricSpace>::UserData) {
        let distance = needle.distance(&node.vantage_point, user_data);

        if distance < best_candidate.distance {
            best_candidate.distance = distance;
            best_candidate.idx = node.idx;
        }

        // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
        if distance < node.radius {
            if let Some(ref near) = node.near {
                Self::search_node(&*near, needle, best_candidate, user_data);
            }
            // The best node (final answer) may be just ouside the radius, but not farther than
            // the best distance we know so far. The search_node above should have narrowed
            // best_candidate.distance, so this path is rarely taken.
            if let Some(ref far) = node.far {
                if distance >= node.radius - best_candidate.distance {
                    Self::search_node(&*far, needle, best_candidate, user_data);
                }
            }
        } else {
            if let Some(ref far) = node.far {
                Self::search_node(&*far, needle, best_candidate, user_data);
            }
            if let Some(ref near) = node.near {
                if distance <= node.radius + best_candidate.distance {
                    Self::search_node(&*near, needle, best_candidate, user_data);
                }
            }
        }
    }

    /**
     * Finds item closest to given needle (that can be any item) and returns *index* of the item in items array from vp_init.
     *
     * @param  needle       The query.
     * @return              Index of the nearest item found and the distance from the nearest item
     */
    pub fn find_nearest(&'a self, needle: &Item) -> (usize, <Item as MetricSpace>::Distance) {
        let mut best_candidate = Tmp{
            distance: <<Item as MetricSpace>::Distance as MaximumValue>::max(),
            idx: 0,
        };
        Self::search_node(&self.root, needle, &mut best_candidate, self.user_data);

        (best_candidate.idx, best_candidate.distance)
    }
}

// Test

#[cfg(test)]
#[derive(Copy, Clone)]
struct Foo(f32);

#[cfg(test)]
impl MetricSpace for Foo {
    type UserData = ();
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        (self.0 - other.0).abs()
    }
}

#[cfg(test)]
#[derive(Copy, Clone)]
struct Bar(f32);

#[cfg(test)]
impl MetricSpace for Bar {
    type UserData = usize;

    fn distance(&self, other: &Self, user_data: &Self::UserData) -> Self::Distance {
        assert_eq!(12345, *user_data);

        (self.0 - other.0).abs()
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
    let bars = [Bar(1.0), Bar(1.5), Bar(2.0)];
    let magic = 12345;
    let vp = Tree::new_with_user_data(&bars, &magic);

    assert_eq!((1, 0.0), vp.find_nearest(&Bar(1.5)));
}
