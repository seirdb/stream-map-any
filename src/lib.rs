//! Allows merging async Streams of different output type.
//!
//! It's very similar to Tokio's [`StreamMap`], except that it doesn't require the streams to have the
//! same output type.
//! This can be usefull when you don't know what type of streams should be combined, acting as a
//! runtime dynamic select.
//!
//! # Not a zero-cost-abstraction
//! Since we don't know what types of outputs the streams will generate, the generated output will
//! be a `StreamMapAnyVariant`, a newtype around `Box<dyn Any>`. As a result, we rely on dynamic
//! dispatching to transform it back into the desired output.
//! Benching shows that it's __2x__ as slow as a [`StreamMap`] or a [`select`] macro implementation.
//!
//! [`StreamMap`]: https://docs.rs/tokio/*/tokio/stream/struct.StreamMap.html
//! [`select`]: https://docs.rs/tokio/*/tokio/macro.select.html

use futures::stream::{Stream, StreamExt};
use std::any::Any;
use std::borrow::Borrow;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Combines streams with different output types into one.
pub struct StreamMapAny<K> {
    streams: Vec<(K, BoxedStream)>,
    last_position: usize,
}

/// Newtype around a Boxed Any.
#[derive(Debug)]
pub struct StreamMapAnyVariant(Box<dyn Any>);

struct BoxedStream(Box<dyn Stream<Item = Box<dyn Any>> + Unpin>);

impl<K> StreamMapAny<K> {
    pub const fn new() -> Self {
        Self {
            streams: Vec::new(),
            last_position: 0,
        }
    }

    /// Insert a new stream into the map with a given key.
    ///
    /// If that key is already in use by another stream, that stream will get dropped.
    pub fn insert<S>(&mut self, key: K, stream: S)
    where
        S: Stream + Unpin + 'static,
        S::Item: Any,
        K: Eq,
    {
        let boxed = BoxedStream::new(stream);
        self.streams.push((key, boxed));
    }

    /// Remove a stream from the map with a given key.
    ///
    /// The stream will get dropped.
    pub fn remove<Q>(&mut self, k: &Q)
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        for i in 0..self.streams.len() {
            if self.streams[i].0.borrow() == k {
                self.streams.swap_remove(i);
            }
        }
    }
}

impl<K> StreamMapAny<K>
where
    K: Unpin + Clone,
{
    fn poll_streams(&mut self, cx: &mut Context) -> Poll<Option<(K, StreamMapAnyVariant)>> {
        let start = self.last_position.wrapping_add(1) % self.streams.len();
        let mut idx = start;
        self.last_position = idx;

        for _ in 0..self.streams.len() {
            let (id, stream) = &mut self.streams[idx];

            match Pin::new(stream).poll_next(cx) {
                Poll::Ready(Some(data)) => {
                    return Poll::Ready(Some((id.clone(), StreamMapAnyVariant(data))));
                }
                Poll::Ready(None) => {
                    self.streams.swap_remove(idx);
                    if idx == self.streams.len() {
                        idx = 0;
                    } else if idx < start && start <= self.streams.len() {
                        idx = idx.wrapping_add(1) % self.streams.len();
                    }
                }
                Poll::Pending => {
                    idx = idx.wrapping_add(1) % self.streams.len();
                }
            }
        }

        if self.streams.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

impl<K> Stream for StreamMapAny<K>
where
    K: Unpin + Clone,
{
    type Item = (K, StreamMapAnyVariant);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.streams.is_empty() {
            return Poll::Ready(None);
        }

        self.poll_streams(cx)
    }
}

impl StreamMapAnyVariant {
    /// Retrieve the value if the type matches T.
    ///
    /// If it doesn't match, the variant will be returned as Err.
    pub fn value<T>(self: Self) -> Result<T, Self>
    where
        T: Any,
    {
        self.0.downcast().map(|v| *v).map_err(Self)
    }

    /// Retrieve a boxed value if the type matches T.
    ///
    /// If it doesn't match, the variant will be returned as Err.
    pub fn boxed_value<T>(self: Self) -> Result<Box<T>, Self>
    where
        T: Any,
    {
        self.0.downcast().map_err(Self)
    }

    /// Retrieve the containing boxed Any.
    pub fn as_boxed_any(self: Self) -> Box<dyn Any> {
        self.0
    }
}

impl BoxedStream {
    fn new<S>(s: S) -> Self
    where
        S: Stream + Unpin + 'static,
        S::Item: Any,
    {
        let stream = s.map(|o| {
            let v: Box<dyn Any> = Box::new(o);
            v
        });
        Self(Box::new(stream))
    }
}

impl Stream for BoxedStream {
    type Item = Box<dyn Any>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut *self.0).poll_next(cx)
    }
}
