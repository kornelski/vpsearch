/// Newtype
#[derive(Clone)]
struct LotsaDimensions<'a>(&'a [u8; 64]);

impl vpsearch::MetricSpace for LotsaDimensions<'_> {
    type UserData = ();
    type Distance = f64;
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        let dist_squared = self.0.iter().copied().zip(other.0.iter().copied())
            .map(|(a, b)| {
                (i32::from(a) - i32::from(b)).pow(2) as u32
            }).sum::<u32>();
        f64::from(dist_squared).sqrt() // sqrt is required
    }
}

fn main() {
    let source_data = [[0; 64], [5; 64], [10; 64]];
    let reference_data: Vec<_> = source_data.iter().map(LotsaDimensions).collect();
    let vp = vpsearch::Tree::new(&reference_data);
    let (index, dist) = vp.find_nearest(&LotsaDimensions(&[6; 64]));
    println!("The element at {index} is the nearest, off by {dist}");
}
