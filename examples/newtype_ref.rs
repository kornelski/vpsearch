/// Newtype
#[derive(Clone)]
struct LotsaDimensions<'a>(&'a [u8; 64]);

impl<'a> vpsearch::MetricSpace for LotsaDimensions<'a> {
    type UserData = ();
    type Distance = f64;
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        let dist_squared = self.0.iter().copied().zip(other.0.iter().copied())
            .map(|(a, b)| {
                (a as i32 - b as i32).pow(2) as u32
            }).sum::<u32>();
        (dist_squared as f64).sqrt() // sqrt is required
    }
}

fn main() {
    let source_data = vec![[0; 64], [5; 64], [10; 64]];
    let reference_data: Vec<_> = source_data.iter().map(LotsaDimensions).collect();
    let vp = vpsearch::Tree::new(&reference_data);
    let (index, dist) = vp.find_nearest(&LotsaDimensions(&[6; 64]));
    println!("The element at {} is the nearest, off by {}", index, dist);
}
