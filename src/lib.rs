use futures::stream::{Stream, StreamExt};
use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) struct BoxedStream(Box<dyn Stream<Item = Box<dyn Any>> + Unpin>);

impl BoxedStream {
    pub(crate) fn new<S>(s: S) -> Self
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

pub struct StreamMapAny<K> {
    streams: Vec<(K, BoxedStream)>,
    last_position: usize,
}

impl<K> StreamMapAny<K> {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            last_position: 0,
        }
    }

    pub fn insert<S>(&mut self, key: K, stream: S)
    where
        S: Stream + Unpin + 'static,
        S::Item: Any,
        K: Hash + Eq,
    {
        let boxed = BoxedStream::new(stream);
        self.streams.push((key, boxed));
    }

    pub fn remove<Q>(&mut self, k: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
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
    pub fn poll_streams(&mut self, cx: &mut Context) -> Poll<Option<(K, StreamMapAnyVariant)>> {
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

#[derive(Debug)]
pub struct StreamMapAnyVariant(Box<dyn Any>);

/*
impl<U> std::convert::TryFrom<StreamMapAnyVariant> for U
where
    U: Any,
{
    type Error = StreamMapAnyVariant;

    fn try_from(value: StreamMapAnyVariant) -> Result<Self, Self::Error> {
        todo!()
    }
}
*/

impl StreamMapAnyVariant {
    pub fn get<V>(self: Self) -> Result<V, Self>
    where
        V: Any,
    {
        self.0.downcast().map(|v| *v).map_err(Self)
    }

    pub fn get_boxed<V>(self: Self) -> Result<Box<V>, Self>
    where
        V: Any,
    {
        self.0.downcast().map_err(Self)
    }

    pub fn as_boxed_any(self: Self) -> Box<dyn Any> {
        self.0
    }
}
