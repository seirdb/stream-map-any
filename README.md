# stream-map-any

Allows merging async Streams of different output type.

It's very similar to Tokio's `StreamMap`, except that it doesn't require the streams to have the
same output type.
This can be useful when you don't know what type of streams should be combined, acting as a
runtime dynamic select.

## Not a zero-cost-abstraction
Since we don't know what types of outputs the streams will generate, the generated output will
be a `StreamMapAnyVariant`, a newtype around `Box<dyn Any>`. As a result, we rely on dynamic
dispatching to transform it back into the desired output.
Benching shows that it's __2x__ as slow as a `StreamMap` or Tokio's `select` macro.

## Example

To get started, add the following to `Cargo.toml`.

```toml
stream-map-any = "0.2"
```

Merging of 2 streams:

```rust,no_run
use futures::channel::mpsc::channel;
use futures::executor::block_on;
use futures::stream::{self, StreamExt};
use stream_map_any::StreamMapAny;

fn main() {
    let int_stream = stream::iter(vec![1; 10]);
    let (mut tx, rx) = channel::<String>(100);

    let mut merge = StreamMapAny::new();
    merge.insert(0, int_stream);
    merge.insert(1, rx);

    std::thread::spawn(move || {
        tx.try_send("hello world".into()).unwrap();
    });

    block_on(async move {
        loop {
            match merge.next().await {
                Some((0, val)) => {
                    let _val: i32 = val.value().unwrap();
                }
                Some((1, val)) => {
                    let _val: String = val.value().unwrap();
                }
                Some(_) => panic!("unexpected key"),
                None => break,
            }
        }
    });
}
```

Further info in the [API Docs](https://docs.rs/stream-map-any/0.2.0/stream_map_any/).

## License

<sup>
Licensed under <a href="LICENSE">MIT license</a>.
</sup>
