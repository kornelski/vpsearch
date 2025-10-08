struct WorkAroundRustOrphanRules;

impl vpsearch::MetricSpace<WorkAroundRustOrphanRules> for Vec<u8> {
    type UserData = ();
    type Distance = f64;
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        let dist_squared = self.iter().copied().zip(other.iter().copied())
            .map(|(a, b)| {
                (i32::from(a) - i32::from(b)).pow(2) as u32
            }).sum::<u32>();
        f64::from(dist_squared).sqrt() // sqrt is required
    }
}

fn main() {
    let source_data = vec![vec![0; 64], vec![5; 64], vec![10; 64]];
    let vp = vpsearch::Tree::new(&source_data);
    let (index, dist) = vp.find_nearest(&vec![6; 64]);
    println!("The element at {index} is the nearest, off by {dist}");
}
