use vpsearch::{BestCandidate, MetricSpace};

use std::collections::HashSet;

#[derive(Clone, Debug)]
struct PointN {
    data: Vec<f32>,
}

/// Point structure that will end up in the tree
impl PointN {
    pub fn new(data: &Vec<f32>) -> Self {
        PointN { data: data.clone() }
    }
}

/// The searching function
impl MetricSpace for PointN {
    type UserData = ();
    type Distance = f32;

    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        self.data
            .iter()
            .zip(other.data.iter())
            .map(|(s, o)| (s - o).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

/// Add custom search for finding the index of the N nearest points
struct CountBasedNeighborhood<Item: MetricSpace<Impl>, Impl> {
    // Max amount of items
    max_item_count: usize,
    // The max distance we have observed so far
    max_observed_distance: Item::Distance,
    // A list of indexes no longer than max_item_count sorted by distance
    distance_x_index: Vec<(Item::Distance, usize)>,
}

impl<Item: MetricSpace<Impl>, Impl> CountBasedNeighborhood<Item, Impl> {
    /// Helper function for creating the CountBasedNeighborhood struct.
    /// Here `item_count` is the amount of items returned, the k in knn.
    fn new(item_count: usize) -> Self {
        CountBasedNeighborhood {
            max_item_count: item_count,
            max_observed_distance: <Item::Distance as Default>::default(),
            distance_x_index: Vec::<(Item::Distance, usize)>::new(),
        }
    }

    /// Insert a single index in the correct possition given that the
    /// `distance_x_index` is already sorted.
    fn insert_index(&mut self, index: usize, distance: Item::Distance) {
        // Add the new item at the end of the list.
        self.distance_x_index.push((distance, index));
        // We only need to sort lists with more than one entry
        if self.distance_x_index.len() > 1 {
            // Start indexing at the end of the vector. Note that len() is 1 indexed.
            let mut n = self.distance_x_index.len() - 1;
            // at n is further than n -1 we swap the two.
            // Prefrom a single insertion sort pass. If the distance of the element
            while n > 0 && self.distance_x_index[n].0 < self.distance_x_index[n - 1].0 {
                self.distance_x_index.swap(n, n - 1);
                n = n - 1;
            }
            self.distance_x_index.truncate(self.max_item_count);
        }
        // Update the max observed distance, unwrap is safe because this function
        // inserts a point and the `max_item_count` is more then 0.
        self.max_observed_distance = self.distance_x_index.last().unwrap().0
    }
}

/// Best candidate definitions that tracks of the index all the points
/// within the radius of `distance` as specified in the `RadiusBasedNeighborhood`.
impl<Item: MetricSpace<Impl> + Clone, Impl> BestCandidate<Item, Impl>
    for CountBasedNeighborhood<Item, Impl>
{
    type Output = HashSet<usize>;

    #[inline]
    fn consider(
        &mut self,
        _: &Item,
        distance: Item::Distance,
        candidate_index: usize,
        _: &Item::UserData,
    ) {
        // Early out, no need to do track any points if the max return size is 0
        if self.max_item_count == 0 {
            return;
        }

        // If the distance is lower than the max_observed distance we
        // need to add that index into the sorted_ids and update the
        // `max_observed_distance`. If the sorted_ids is already at max
        // capacity we drop the point with the max distance and find
        // out what the new `max_observed_distance` is by looking at
        // the last entry in the `distance_x_index` vector. We also
        // include the point if the `distance_x_index` is not full yet.
        if distance < self.max_observed_distance
            || self.distance_x_index.len() < self.max_item_count
        {
            self.insert_index(candidate_index, distance);
        }
    }

    #[inline]
    fn distance(&self) -> Item::Distance {
        // return distance of the Nth farthest as we have currently observed it.
        // All other points currently in the state will be closer than this.
        self.max_observed_distance
    }

    fn result(self, _: &Item::UserData) -> Self::Output {
        // Convert the sorted indexes into a hash set droping the distance value.
        self.distance_x_index
            .into_iter()
            .map(|(_, index)| index)
            .collect::<HashSet<usize>>()
    }
}

fn main() {
    let points = vec![
        PointN::new(&vec![2.0, 3.0]),
        PointN::new(&vec![0.0, 1.0]),
        PointN::new(&vec![4.0, 5.0]),
    ];
    let tree = vpsearch::Tree::new(&points);

    // Search with a neigboord size of 1, expect a single points to be returned
    let actual = tree.find_nearest_custom(
        &PointN::new(&vec![1.0, 2.0]),
        &(),
        CountBasedNeighborhood::new(1),
    );
    assert_eq!(actual.len(), 1);

    // Search with a neigboord size of 2, expect a two points to be returned
    let expected = [0, 1].iter().cloned().collect::<HashSet<usize>>();
    let actual = tree.find_nearest_custom(
        &PointN::new(&vec![1.0, 2.0]),
        &(),
        CountBasedNeighborhood::new(2),
    );
    assert_eq!(actual, expected);

    // Search with a neigboord size of 10, expect all points to be returned
    let expected = [0, 1, 2].iter().cloned().collect::<HashSet<usize>>();
    let actual = tree.find_nearest_custom(
        &PointN::new(&vec![1.0, 2.0]),
        &(),
        CountBasedNeighborhood::new(10),
    );
    assert_eq!(actual, expected)
}
