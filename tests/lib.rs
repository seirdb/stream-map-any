use futures::channel::mpsc::channel;
use futures::executor::block_on;
use futures::stream::{self, StreamExt};
use stream_map_any::StreamMapAny;

#[test]
fn single_stream() {
    let stream = stream::iter(vec![100]);

    let mut merge = StreamMapAny::new();

    merge.insert(0, stream);

    block_on(async move {
        loop {
            match merge.next().await {
                Some((0, msg)) => {
                    let val: i32 = msg.value().unwrap();
                    assert_eq!(val, 100);
                }
                Some(_) => panic!("unexpected key"),
                None => break,
            }
        }
    });
}

#[test]
fn repeat_stream() {
    let stream = stream::repeat(1);

    let mut merge = StreamMapAny::new();

    merge.insert(0, stream);

    block_on(async move {
        let mut count = 0;
        for _ in 0u8..100 {
            match merge.next().await {
                Some((0, msg)) => {
                    let val: i32 = msg.value().unwrap();
                    count += val;
                }
                Some(_) => panic!("unexpected key"),
                None => break,
            }
        }
        assert_eq!(count, 100);
    });
}

#[test]
fn mixed_stream_types() {
    let int_stream = stream::iter(vec![1; 10]);
    let (mut tx, rx) = channel::<String>(100);

    let mut merge = StreamMapAny::new();

    merge.insert(0, int_stream);
    merge.insert(1, rx);

    std::thread::spawn(move || {
        tx.try_send("hello".into()).unwrap();
    });

    block_on(async move {
        let mut count = 0;
        let mut hello_msg = None;
        loop {
            match merge.next().await {
                Some((0, msg)) => {
                    let val: i32 = msg.value().unwrap();
                    count += val;
                }
                Some((1, msg)) => {
                    let val: String = msg.value().unwrap();
                    hello_msg = Some(val);
                }
                Some(_) => panic!("unexpected key"),
                None => break,
            }
        }

        let hello_msg = hello_msg.unwrap();
        assert_eq!(hello_msg, "hello");
        assert_eq!(count, 10);
    });
}
