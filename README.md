# VP-tree nearest neighbor search

A relatively simple and readable Rust implementation of Vantage Point tree search algorithm.

The VP tree algorithm doesn't need to know coordinates of items, only distances between them. It can efficiently search multi-dimensional spaces and abstract things as long as you can define similarity between them (e.g. points, colors, and even images).

Please see [the API reference](https://docs.rs/vpsearch) or [examples](https://github.com/kornelski/vpsearch/tree/rust/examples) for details.

```Rust
#[derive(Copy, Clone)]
struct Point {
    x: f32, y: f32,
}

impl vpsearch::MetricSpace for Point {
    type UserData = ();
    type Distance = f32;

    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        let dx = self.x - other.x;
        let dy = self.y - other.y;

        // sqrt is required! The algorithm can't work with squared distances.
        (dx*dx + dy*dy).sqrt()
    }
}

fn main() {
    let points = vec![Point{x:2.0,y:3.0}, Point{x:0.0,y:1.0}, Point{x:4.0,y:5.0}];

    let vp = vpsearch::Tree::new(&points);
    let (index, _) = vp.find_nearest(&Point{x:1.0,y:2.0});

    println!("The nearest point is at ({}, {})", points[index].x, points[index].y);
}
```
