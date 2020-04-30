use criterion::{criterion_group, criterion_main, Criterion};
use futures::executor::block_on;
use futures::stream::{self, StreamExt};
use stream_map_any::StreamMapAny;

fn bench_merge_single_stream(c: &mut Criterion) {
    let stream = stream::repeat(1);

    let mut merge = StreamMapAny::new();

    merge.insert(0, stream);

    c.bench_function("merged_single_stream", |b| {
        b.iter(|| {
            block_on(async {
                let mut count = 0;
                for _ in 0u32..100 {
                    match merge.next().await {
                        Some((0, msg)) => {
                            let val: i32 = msg.get().unwrap();
                            count += val;
                        }
                        Some(_) => panic!("unexpected key"),
                        None => break,
                    }
                }

                assert_eq!(100, count);
            });
        })
    });
}

fn bench_single_stream(c: &mut Criterion) {
    let mut stream = stream::repeat(1);

    c.bench_function("single_stream", |b| {
        b.iter(|| {
            block_on(async {
                let mut count = 0;
                for _ in 0u32..100 {
                    match stream.next().await {
                        Some(val) => count += val,
                        None => break,
                    }
                }

                assert_eq!(100, count);
            });
        })
    });
}

fn bench_merge_two_streams(c: &mut Criterion) {
    let stream = stream::repeat(1i32);
    let other_stream = stream::repeat(1u32);

    let mut merge = StreamMapAny::new();

    merge.insert(0, stream);
    merge.insert(1, other_stream);

    c.bench_function("bench_merge_two_stream", |b| {
        b.iter(|| {
            block_on(async {
                let mut count = 0;
                let mut other_count = 0;
                loop {
                    match merge.next().await {
                        Some(_) if count == 50 && other_count == 50 => break,
                        Some((0, msg)) if count != 50 => {
                            let val: i32 = msg.get().unwrap();
                            count += val;
                        }
                        Some((0, _)) => {}
                        Some((1, msg)) if other_count != 50 => {
                            let val: u32 = msg.get().unwrap();
                            other_count += val;
                        }
                        Some((1, _)) => {}
                        Some((k, _)) => panic!("invalid key: {}", k),
                        None => break,
                    }
                }

                assert_eq!(50, count);
                assert_eq!(50, other_count);
            });
        })
    });
}

fn bench_tokio_streammap_two_streams(c: &mut Criterion) {
    let stream = stream::repeat(1i32);
    let other_stream = stream::repeat(1i32);

    let mut merge = tokio::stream::StreamMap::new();

    merge.insert(0, stream);
    merge.insert(1, other_stream);

    c.bench_function("bench_tokio_streammap_two_streams", |b| {
        b.iter(|| {
            block_on(async {
                let mut count = 0;
                let mut other_count = 0;
                loop {
                    match merge.next().await {
                        Some(_) if count == 50 && other_count == 50 => break,
                        Some((0, val)) if count != 50 => {
                            count += val;
                        }
                        Some((0, _)) => {}
                        Some((1, val)) if other_count != 50 => {
                            other_count += val;
                        }
                        Some((1, _)) => {}
                        Some((k, _)) => panic!("invalid key: {}", k),
                        None => break,
                    }
                }

                assert_eq!(50, count);
                assert_eq!(50, other_count);
            });
        })
    });
}

fn bench_tokio_select_two_streams(c: &mut Criterion) {
    let mut stream = stream::repeat(1i32);
    let mut other_stream = stream::repeat(1i32);

    c.bench_function("bench_tokio_select_two_streams", |b| {
        b.iter(|| {
            block_on(async {
                let mut count = 0;
                let mut other_count = 0;

                loop {
                    tokio::select! {
                        Some(val) = stream.next(), if count != 50 => {
                                count += val;
                        }
                        Some(val) = other_stream.next(), if other_count != 50 => {
                                other_count += val;
                        }
                    }
                    if count == 50 && other_count == 50 {
                        break;
                    }
                }

                assert_eq!(50, count);
                assert_eq!(50, other_count);
            });
        })
    });
}

criterion_group!(
    single_stream_benches,
    bench_single_stream,
    bench_merge_single_stream
);

criterion_group!(
    two_stream_benches,
    bench_merge_two_streams,
    bench_tokio_streammap_two_streams,
    bench_tokio_select_two_streams,
);

criterion_main!(single_stream_benches, two_stream_benches);
