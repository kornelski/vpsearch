struct WorkAroundRustOrphanRules;

impl<'a> vpsearch::MetricSpace<WorkAroundRustOrphanRules> for Vec<u8> {
    type UserData = ();
    type Distance = f64;
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        let dist_squared = self.iter().copied().zip(other.iter().copied())
            .map(|(a, b)| {
                (a as i32 - b as i32).pow(2) as u32
            }).sum::<u32>();
        (dist_squared as f64).sqrt() // sqrt is required
    }
}

fn main() {
    let source_data = vec![vec![0; 64], vec![5; 64], vec![10; 64]];
    let vp = vpsearch::Tree::new(&source_data);
    let (index, dist) = vp.find_nearest(&vec![6; 64]);
    println!("The element at {} is the nearest, off by {}", index, dist);
}
