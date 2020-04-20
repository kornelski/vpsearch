//! A relatively simple and readable Rust implementation of Vantage Point tree search algorithm.
//!
//! The VP tree algorithm doesn't need to know coordinates of items, only distances between them. It can efficiently search multi-dimensional spaces and abstract things as long as you can define similarity between them (e.g. points, colors, and even images).
//!
//! [Project page](https://github.com/kornelski/vpsearch).
//!
//!
//! **This algorithm does not work with squared distances. When implementing Euclidean distance, you *MUST* use `sqrt()`**. You really really can't use that optimization. There's no way around it. Vantage Point trees require [metric spaces](https://en.wikipedia.org/wiki/Metric_space).
//!
//! ```rust
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
//!
//! ```rust
//! #[derive(Clone)]
//! struct LotsaDimensions<'a>(&'a [u8; 64]);
//!
//! impl<'a> vpsearch::MetricSpace for LotsaDimensions<'a> {
//!     type UserData = ();
//!     type Distance = f64;
//!
//!     fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
//!         let dist_squared = self.0.iter().copied().zip(other.0.iter().copied())
//!             .map(|(a, b)| {
//!                 (a as i32 - b as i32).pow(2) as u32
//!             }).sum::<u32>();
//!
//!         (dist_squared as f64).sqrt() // sqrt is required
//!     }
//! }
//!
//! fn main() {
//!     let points = vec![LotsaDimensions(&[0; 64]), LotsaDimensions(&[5; 64]), LotsaDimensions(&[10; 64])];
//!     let vp = vpsearch::Tree::new(&points);
//!     let (index, _) = vp.find_nearest(&LotsaDimensions(&[6; 64]));
//!     println!("The {}th element is the nearest", index);
//! }
//! ```



use std::cmp::Ordering;
use std::ops::Add;
use std::marker::Sized;
use num_traits::Bounded;

#[cfg(test)]
mod test;
mod debug;

#[doc(hidden)]
pub struct Owned<T>(T);

/// Elements you're searching for must be comparable using this trait.
///
/// You can ignore `UserImplementationType` if you're implementing `MetricSpace` for your custom type.
/// However, if you're implementing `MetricSpace` for a type from std or another crate, then you need
/// to uniquely identify your implementation (that's because of Rust's Orphan Rules).
///
/// ```rust,ignore
/// impl MetricSpace for MyInt {/*…*/}
///
/// /// That dummy struct disambiguates between yours and everyone else's impl for a tuple:
/// struct MyXYCoordinates;
/// impl MetricSpace<MyXYCoordinates> for (f32,f32) {/*…*/}
pub trait MetricSpace<UserImplementationType=()> {
    /// This is used as a context for comparisons. Use `()` if the elements already contain all the data you need.
    type UserData;

    /// This is a fancy way of saying it should be `f32` or `u32`
    type Distance: Copy + PartialOrd + Bounded + Add<Output=Self::Distance>;

    /**
     * This function must return distance between two items that meets triangle inequality.
     * Specifically, it **MUST NOT return a squared distance** (you must use sqrt if you use Euclidean distance)
     *
     * @param user_data Whatever you want. Passed from `new_with_user_data_*()`
     */
    fn distance(&self, other: &Self, user_data: &Self::UserData) -> Self::Distance;
}

/// You can implement this if you want to peek at all visited elements
///
/// ```rust
/// # use vpsearch::*;
/// struct Impl;
/// struct ReturnByIndex<I: MetricSpace<Impl>> {
///    distance: I::Distance,
///    idx: usize,
/// }
///
/// impl<Item: MetricSpace<Impl> + Clone> BestCandidate<Item, Impl> for ReturnByIndex<Item> {
///     type Output = (usize, Item::Distance);
///
///     fn consider(&mut self, _: &Item, distance: Item::Distance, candidate_index: usize, _: &Item::UserData) {
///         if distance < self.distance {
///             self.distance = distance;
///             self.idx = candidate_index;
///         }
///     }
///     fn distance(&self) -> Item::Distance {
///         self.distance
///     }
///     fn result(self, _: &Item::UserData) -> Self::Output {
///         (self.idx, self.distance)
///     }
/// }
/// ```
pub trait BestCandidate<Item: MetricSpace<Impl> + Clone, Impl> where Self: Sized {
    /// find_nearest() will return this type
    type Output;

    /// This is a visitor method. If the given distance is smaller than previously seen, keep the item (or its index).
    /// UserData is the same as for `MetricSpace<Impl>`, and it's `()` by default.
    fn consider(&mut self, item: &Item, distance: Item::Distance, candidate_index: usize, user_data: &Item::UserData);

    /// Minimum distance seen so far
    fn distance(&self) -> Item::Distance;

    /// Called once after all relevant nodes in the tree were visited
    fn result(self, user_data: &Item::UserData) -> Self::Output;
}

impl<Item: MetricSpace<Impl> + Clone, Impl> BestCandidate<Item, Impl> for ReturnByIndex<Item, Impl> {
    type Output = (usize, Item::Distance);

    #[inline]
    fn consider(&mut self, _: &Item, distance: Item::Distance, candidate_index: usize, _: &Item::UserData) {
        if distance < self.distance {
            self.distance = distance;
            self.idx = candidate_index;
        }
    }

    #[inline]
    fn distance(&self) -> Item::Distance {
        self.distance
    }

    fn result(self, _: &Item::UserData) -> (usize, Item::Distance) {
        (self.idx, self.distance)
    }
}

const NO_NODE: u32 = u32::max_value();

struct Node<Item: MetricSpace<Impl> + Clone, Impl> {
    near: u32,
    far: u32,
    vantage_point: Item, // Pointer to the item (value) represented by the current node
    radius: Item::Distance,    // How far the `near` node stretches
    idx: u32,             // Index of the `vantage_point` in the original items array
}

/// The VP-Tree.
pub struct Tree<Item: MetricSpace<Impl> + Clone, Impl=(), Ownership=Owned<()>> {
    nodes: Vec<Node<Item, Impl>>,
    root: u32,
    user_data: Ownership,
}

/* Temporary object used to reorder/track distance between items without modifying the orignial items array
   (also used during search to hold the two properties).
*/
struct Tmp<Item: MetricSpace<Impl>, Impl> {
    distance: Item::Distance,
    idx: u32,
}

struct ReturnByIndex<Item: MetricSpace<Impl>, Impl> {
    distance: Item::Distance,
    idx: usize,
}

impl<Item: MetricSpace<Impl>, Impl> ReturnByIndex<Item, Impl> {
    fn new() -> Self {
        ReturnByIndex {
            distance: <Item::Distance as Bounded>::max_value(),
            idx: 0,
        }
    }
}

impl<Item: MetricSpace<Impl, UserData = ()> + Clone, Impl> Tree<Item, Impl, Owned<()>> {

    /**
     * Creates a new tree from items.
     *
     * @see Tree::new_with_user_data_owned
     */
    pub fn new(items: &[Item]) -> Self {
        Self::new_with_user_data_owned(items, ())
    }
}

impl<U, Impl, Item: MetricSpace<Impl, UserData = U> + Clone> Tree<Item, Impl, Owned<U>> {
    /**
     * Finds item closest to given needle (that can be any item) and Output *index* of the item in items array from `new()`.
     *
     * @param  needle       The query.
     * @return              Index of the nearest item found and the distance from the nearest item
     */

    #[inline]
    pub fn find_nearest(&self, needle: &Item) -> (usize, Item::Distance) {
        self.find_nearest_with_user_data(needle, &self.user_data.0)
    }
}

impl<Item: MetricSpace<Impl> + Clone, Ownership, Impl> Tree<Item, Impl, Ownership> {
    fn sort_indexes_by_distance(vantage_point: Item, indexes: &mut [Tmp<Item, Impl>], items: &[Item], user_data: &Item::UserData) {
        for i in indexes.iter_mut() {
            i.distance = vantage_point.distance(&items[i.idx as usize], user_data);
        }
        indexes.sort_by(|a, b| if a.distance < b.distance {Ordering::Less} else {Ordering::Greater});
    }

    fn create_node(indexes: &mut [Tmp<Item, Impl>], nodes: &mut Vec<Node<Item, Impl>>, items: &[Item], user_data: &Item::UserData) -> u32 {
        if indexes.len() == 0 {
            return NO_NODE;
        }

        if indexes.len() == 1 {
            let node_idx = nodes.len();
            nodes.push(Node{
                near: NO_NODE, far: NO_NODE,
                vantage_point: items[indexes[0].idx as usize].clone(),
                idx: indexes[0].idx,
                radius: <Item::Distance as Bounded>::max_value(),
            });
            return node_idx as u32;
        }

        let ref_idx = indexes[0].idx;

        // Removes the `ref_idx` item from remaining items, because it's included in the current node
        let rest = &mut indexes[1..];

        Self::sort_indexes_by_distance(items[ref_idx as usize].clone(), rest, items, user_data);

        // Remaining items are split by the median distance
        let half_idx = rest.len()/2;

        let (near_indexes, far_indexes) = rest.split_at_mut(half_idx);
        let vantage_point = items[ref_idx as usize].clone();
        let radius = far_indexes[0].distance;

        // push first to reserve space before its children
        let node_idx = nodes.len();
        nodes.push(Node{
            vantage_point,
            idx: ref_idx,
            radius,
            near: NO_NODE,
            far: NO_NODE,
        });

        let near = Self::create_node(near_indexes, nodes, items, user_data);
        let far = Self::create_node(far_indexes, nodes, items, user_data);
        nodes[node_idx].near = near;
        nodes[node_idx].far = far;
        node_idx as u32
    }
}

impl<Item: MetricSpace<Impl> + Clone, Impl> Tree<Item, Impl, Owned<Item::UserData>> {
    /**
     * Create a Vantage Point tree for fast nearest neighbor search.
     *
     * @param  items        Array of items that will be searched.
     * @param  user_data    Reference to any object that is passed down to item.distance()
     */
    pub fn new_with_user_data_owned(items: &[Item], user_data: Item::UserData) -> Self {
        let mut nodes = Vec::new();
        let root = Self::create_root_node(items, &mut nodes, &user_data);
        Tree {
            root,
            nodes,
            user_data: Owned(user_data),
        }
    }
}

impl<Item: MetricSpace<Impl> + Clone, Impl> Tree<Item, Impl, ()> {
    /// The tree doesn't have to own the UserData. You can keep passing it to find_nearest().
    pub fn new_with_user_data_ref(items: &[Item], user_data: &Item::UserData) -> Self {
        let mut nodes = Vec::new();
        let root = Self::create_root_node(items, &mut nodes, &user_data);
        Tree {
            root,
            nodes,
            user_data: (),
        }
    }

    #[inline]
    pub fn find_nearest(&self, needle: &Item, user_data: &Item::UserData) -> (usize, Item::Distance) {
        self.find_nearest_with_user_data(needle, user_data)
    }
}

impl<Item: MetricSpace<Impl> + Clone, Ownership, Impl> Tree<Item, Impl, Ownership> {
    fn create_root_node(items: &[Item], nodes: &mut Vec<Node<Item, Impl>>, user_data: &Item::UserData) -> u32 {
        assert!(items.len() < (u32::max_value()/2) as usize);

        let mut indexes: Vec<_> = (0..items.len() as u32).map(|i| Tmp{
            idx: i, distance: <Item::Distance as Bounded>::max_value(),
        }).collect();

        Self::create_node(&mut indexes[..], nodes, items, user_data) as u32
    }

    fn search_node<B: BestCandidate<Item, Impl>>(node: &Node<Item, Impl>, nodes: &[Node<Item, Impl>], needle: &Item, best_candidate: &mut B, user_data: &Item::UserData) {
        let distance = needle.distance(&node.vantage_point, user_data);

        best_candidate.consider(&node.vantage_point, distance, node.idx as usize, user_data);

        // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
        if distance < node.radius {
            // No-node case uses out-of-bounds index, so this reuses a safe bounds check as the "null" check
            if let Some(near) = nodes.get(node.near as usize) {
                Self::search_node(near, nodes, needle, best_candidate, user_data);
            }
            // The best node (final answer) may be just ouside the radius, but not farther than
            // the best distance we know so far. The search_node above should have narrowed
            // best_candidate.distance, so this path is rarely taken.
            if let Some(far) = nodes.get(node.far as usize) {
                if distance + best_candidate.distance() >= node.radius {
                    Self::search_node(far, nodes, needle, best_candidate, user_data);
                }
            }
        } else {
            if let Some(far) = nodes.get(node.far as usize) {
                Self::search_node(far, nodes, needle, best_candidate, user_data);
            }
            if let Some(near) = nodes.get(node.near as usize) {
                if distance <= node.radius + best_candidate.distance() {
                    Self::search_node(near, nodes, needle, best_candidate, user_data);
                }
            }
        }
    }

    #[inline]
    fn find_nearest_with_user_data(&self, needle: &Item, user_data: &Item::UserData) -> (usize, Item::Distance) {
        self.find_nearest_custom(needle, user_data, ReturnByIndex::new())
    }

    #[inline]
    /// All the bells and whistles version. For best_candidate implement `BestCandidate<Item, Impl>` trait.
    pub fn find_nearest_custom<ReturnBy: BestCandidate<Item, Impl>>(&self, needle: &Item, user_data: &Item::UserData, mut best_candidate: ReturnBy) -> ReturnBy::Output {
        Self::search_node(&self.nodes[self.root as usize], &self.nodes, needle, &mut best_candidate, user_data);

        best_candidate.result(user_data)
    }
}
