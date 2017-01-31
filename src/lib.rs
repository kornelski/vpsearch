//! A relatively simple and readable Rust implementation of Vantage Point tree search algorithm.
//!
//! The VP tree algorithm doesn't need to know coordinates of items, only distances between them. It can efficiently search multi-dimensional spaces and abstract things as long as you can define similarity between them (e.g. points, colors, and even images).
//!
//! [Project page](https://github.com/pornel/vpsearch).
//!
//! ```rust
//! extern crate vpsearch;
//!
//! #[derive(Copy, Clone)]
//! struct Point {
//!     x: f32, y: f32,
//! }
//!
//! impl vpsearch::MetricSpace for Point {
//!     type UserData = ();
//!     type Distance = f32;
//!
//!     fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
//!         let dx = self.x - other.x;
//!         let dy = self.y - other.y;
//!         (dx*dx + dy*dy).sqrt() // sqrt is required
//!     }
//! }
//!
//! fn main() {
//!     let points = vec![Point{x:2.0,y:3.0}, Point{x:0.0,y:1.0}, Point{x:4.0,y:5.0}];
//!     let vp = vpsearch::Tree::new(&points);
//!     let (index, _) = vp.find_nearest(&Point{x:1.0,y:2.0});
//!     println!("The nearest point is at ({}, {})", points[index].x, points[index].y);
//! }
//! ```

extern crate num_traits;

use std::cmp::Ordering;
use std::ops::Add;
use num_traits::Bounded;

#[cfg(test)]
mod test;

#[doc(hidden)]
pub struct UserDataByRef;
#[doc(hidden)]
pub struct Owned<T>(T);

/// Elements you're searching for must be comparable using this trait
pub trait MetricSpace {
    /// This is used as a context for comparisons. Use `()` if the elements already contain all the data you need.
    type UserData;

    /// This is a fancy way of saying it should be `f32` or `u32`
    type Distance: Copy + PartialOrd + Bounded + Add<Output=Self::Distance>;

    /**
     * This function must return distance between two items that meets triangle inequality.
     * Specifically, it can't return squared distance (you must use sqrt if you use Euclidean distance)
     *
     * @param user_data Whatever you want. Passed from `new_with_user_data_*()`
     */
    fn distance(&self, other: &Self, user_data: &Self::UserData) -> Self::Distance;
}

pub trait BestCandidate<T> {
    fn new() -> Self;
    fn consider(&mut self, distance: T, candidate_index: usize);
}

impl<Item: MetricSpace> BestCandidate<<Item as MetricSpace>::Distance> for Tmp<Item>  {
    fn new() -> Self {
        Tmp {
            distance: <Item::Distance as Bounded>::max_value(),
            idx: 0,
        }
    }

    #[inline]
    fn consider(&mut self, distance: Item::Distance, candidate_index: usize) {
        if distance < self.distance {
            self.distance = distance;
            self.idx = candidate_index;
        }
    }
}

struct Node<Item: MetricSpace + Copy> {
    near: Option<Box<Node<Item>>>,
    far: Option<Box<Node<Item>>>,
    vantage_point: Item, // Pointer to the item (value) represented by the current node
    radius: Item::Distance,    // How far the `near` node stretches
    idx: usize,             // Index of the `vantage_point` in the original items array
}

/// The VP-Tree
pub struct Tree<Item: MetricSpace + Copy, Ownership> {
    root: Node<Item>,
    user_data: Ownership,
}

/* Temporary object used to reorder/track distance between items without modifying the orignial items array
   (also used during search to hold the two properties).
*/
struct Tmp<Item: MetricSpace> {
    distance: Item::Distance,
    idx: usize,
}

impl<Item: MetricSpace<UserData = ()> + Copy> Tree<Item, Owned<()>> {

    /**
     * Creates a new tree from items.
     *
     * @see Tree::new_with_user_data_owned
     */
    pub fn new(items: &[Item]) -> Self {
        Self::new_with_user_data_owned(items, ())
    }
}

impl<U, Item: MetricSpace<UserData = U> + Copy> Tree<Item, Owned<U>> {
    /**
     * Finds item closest to given needle (that can be any item) and returns *index* of the item in items array from `new()`.
     *
     * @param  needle       The query.
     * @return              Index of the nearest item found and the distance from the nearest item
     */
    #[inline]
    pub fn find_nearest(&self, needle: &Item) -> (usize, Item::Distance) {
        self.find_nearest_with_user_data(needle, &self.user_data.0)
    }
}

impl<Item: MetricSpace + Copy, Ownership> Tree<Item, Ownership> {
    fn sort_indexes_by_distance(vantage_point: Item, indexes: &mut [Tmp<Item>], items: &[Item], user_data: &Item::UserData) {
        for i in indexes.iter_mut() {
            i.distance = vantage_point.distance(&items[i.idx], user_data);
        }
        indexes.sort_by(|a, b| if a.distance < b.distance {Ordering::Less} else {Ordering::Greater});
    }

    fn create_node(indexes: &mut [Tmp<Item>], items: &[Item], user_data: &Item::UserData) -> Option<Node<Item>> {
        if indexes.len() == 0 {
            return None;
        }

        if indexes.len() == 1 {
            return Some(Node{
                near: None, far: None,
                vantage_point: items[indexes[0].idx],
                idx: indexes[0].idx,
                radius: <Item::Distance as Bounded>::max_value(),
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
}

impl<Item: MetricSpace + Copy> Tree<Item, Owned<Item::UserData>> {
    /**
     * Create a Vantage Point tree for fast nearest neighbor search.
     *
     * @param  items        Array of items that will be searched.
     * @param  user_data    Reference to any object that is passed down to item.distance()
     */
    pub fn new_with_user_data_owned(items: &[Item], user_data: Item::UserData) -> Self {
        Tree {
            root: Self::create_root_node(items, &user_data),
            user_data: Owned(user_data),
        }
    }
}

impl<Item: MetricSpace + Copy> Tree<Item, UserDataByRef> {
    pub fn new_with_user_data_ref(items: &[Item], user_data: &Item::UserData) -> Self {
        Tree {
            root: Self::create_root_node(items, &user_data),
            user_data: UserDataByRef,
        }
    }

    #[inline]
    pub fn find_nearest(&self, needle: &Item, user_data: &Item::UserData) -> (usize, Item::Distance) {
        self.find_nearest_with_user_data(needle, user_data)
    }
}

use std::fmt::{Debug,Formatter,Error};
impl<Item: Debug + Copy + MetricSpace, Ownership> Debug for Tree<Item, Ownership> {
    fn fmt(&self, f:&mut Formatter) -> Result<(),Error> {
        write!(f, "digraph \"vp tree.dot\" {{\n{:?}}}", self.root)
    }
}

impl<Item: Debug + Copy + MetricSpace> Debug for Node<Item> {
    fn fmt(&self, f:&mut Formatter) -> Result<(),Error> {
        if self.near.is_some() {
            try!(write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.near.as_ref().unwrap().vantage_point));
            try!(self.near.as_ref().unwrap().fmt(f));
        }
        if self.far.is_some() {
            try!(write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.far.as_ref().unwrap().vantage_point));
            try!(self.far.as_ref().unwrap().fmt(f));
        }
        return Ok(());
    }
}

impl<Item: MetricSpace + Copy, Ownership> Tree<Item, Ownership> {
    fn create_root_node(items: &[Item], user_data: &Item::UserData) -> Node<Item> {
        let mut indexes: Vec<_> = (0..items.len()).map(|i| Tmp{
            idx:i, distance: <Item::Distance as Bounded>::max_value(),
        }).collect();

        Self::create_node(&mut indexes[..], items, user_data).unwrap()
    }

    fn search_node(node: &Node<Item>, needle: &Item, best_candidate: &mut Tmp<Item>, user_data: &Item::UserData) {
        let distance = needle.distance(&node.vantage_point, user_data);

        best_candidate.consider(distance, node.idx);

        // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
        if distance < node.radius {
            if let Some(ref near) = node.near {
                Self::search_node(&*near, needle, best_candidate, user_data);
            }
            // The best node (final answer) may be just ouside the radius, but not farther than
            // the best distance we know so far. The search_node above should have narrowed
            // best_candidate.distance, so this path is rarely taken.
            if let Some(ref far) = node.far {
                if distance + best_candidate.distance >= node.radius  {
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

    #[inline]
    fn find_nearest_with_user_data(&self, needle: &Item, user_data: &Item::UserData) -> (usize, Item::Distance) {
        let mut best_candidate = Tmp::new();
        Self::search_node(&self.root, needle, &mut best_candidate, user_data);

        (best_candidate.idx, best_candidate.distance)
    }
}
