use std::cmp::Ordering;

pub type Distance = f32;

pub trait MetricSpace {
    fn distance(&self, other: &Self) -> Distance;
}

struct Node<Item: MetricSpace + Copy> {
    near: Option<Box<Node<Item>>>,
    far: Option<Box<Node<Item>>>,
    vantage_point: Item, // Pointer to the item (value) represented by the current node
    radius: Distance,    // How far the `near` node stretches
    idx: usize,             // Index of the `vantage_point` in the original items array
}

pub struct Handle<Item: MetricSpace + Copy> {
    root: Node<Item>,
}

/* Temporary object used to reorder/track distance between items without modifying the orignial items array
   (also used during search to hold the two properties).
*/
struct Tmp {
    distance: Distance,
    idx: usize,
}

impl<Item: MetricSpace + Copy> Handle<Item> {

    fn sort_indexes_by_distance(vantage_point: Item, indexes: &mut [Tmp], items: &[Item]) {
        for i in indexes.iter_mut() {
            i.distance = vantage_point.distance(&items[i.idx]);
        }
        indexes.sort_by(|a, b| if a.distance < b.distance {Ordering::Less} else {Ordering::Greater});
    }

    fn create_node(indexes: &mut [Tmp], items: &[Item]) -> Option<Node<Item>> {
        if indexes.len() == 0 {
            return None;
        }

        if indexes.len() == 1 {
            return Some(Node{
                near: None, far: None,
                vantage_point: items[indexes[0].idx],
                idx: indexes[0].idx,
                radius: std::f32::MAX,
            });
        }

        let ref_idx = indexes[0].idx;

        // Removes the `ref_idx` item from remaining items, because it's included in the current node
        let rest = &mut indexes[1..];

        Self::sort_indexes_by_distance(items[ref_idx], rest, items);

        // Remaining items are split by the median distance
        let half_idx = rest.len()/2;

        let (near_indexes, far_indexes) = rest.split_at_mut(half_idx);

        Some(Node{
            vantage_point: items[ref_idx],
            idx: ref_idx,
            radius: far_indexes[0].distance,
            near: Self::create_node(near_indexes, items).map(|i| Box::new(i)),
            far: Self::create_node(far_indexes, items).map(|i| Box::new(i)),
        })
    }

    /**
     * Create a Vantage Point tree for fast nearest neighbor search.
     *
     * Note that the callback must return distances that meet triangle inequality.
     * Specifically, it can't return squared distance (you must use sqrt if you use Euclidean distance)
     *
     * @param  items        Array of pointers to items that will be searched. Must not be freed until the tree is freed!
     * @param  num_items    Number of items in the array. Must be > 0
     * @param  get_distance A callback function that will calculdate distance between two items given their pointers.
     * @return              NULL on error or a handle that must be freed with vp_free().
     */
    pub fn new(items: &[Item]) -> Handle<Item> {
        let mut indexes: Vec<_> = (0..items.len()).map(|i| Tmp{
            idx:i, distance:0.0,
        }).collect();

        Handle {
            root: Self::create_node(&mut indexes[..], items).unwrap(),
        }
    }

    fn search_node(node: &Node<Item>, needle: &Item, best_candidate: &mut Tmp) {
        let distance = needle.distance(&node.vantage_point);

        if distance < best_candidate.distance {
            best_candidate.distance = distance;
            best_candidate.idx = node.idx;
        }

        // Recurse towards most likely candidate first to narrow best candidate's distance as soon as possible
        if distance < node.radius {
            if let Some(ref near) = node.near {
                Self::search_node(&*near, needle, best_candidate);
            }
            // The best node (final answer) may be just ouside the radius, but not farther than
            // the best distance we know so far. The search_node above should have narrowed
            // best_candidate.distance, so this path is rarely taken.
            if let Some(ref far) = node.far {
                if distance >= node.radius - best_candidate.distance {
                    Self::search_node(&*far, needle, best_candidate);
                }
            }
        } else {
            if let Some(ref far) = node.far {
                Self::search_node(&*far, needle, best_candidate);
            }
            if let Some(ref near) = node.near {
                if distance <= node.radius + best_candidate.distance {
                    Self::search_node(&*near, needle, best_candidate);
                }
            }
        }
    }

    /**
     * Finds item closest to given needle (that can be any item) and returns *index* of the item in items array from vp_init.
     *
     * @param  handle       VP tree from vp_init(). Must not be NULL.
     * @param  needle       The query.
     * @return              Index of the nearest item found.
     */
    pub fn find_nearest(&self, needle: &Item) -> usize {
        let mut best_candidate = Tmp{
            distance: std::f32::MAX,
            idx: 0,
        };
        Self::search_node(&self.root, needle, &mut best_candidate);
        return best_candidate.idx;
    }
}
