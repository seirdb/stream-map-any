use futures::stream::{Stream, StreamExt};
use std::any::Any;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct BoxedStream(Box<dyn Stream<Item = Box<dyn Any>> + Unpin>);

impl BoxedStream {
    pub fn new<S>(s: S) -> Self
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

pub struct StreamMapAny {
    streams: Vec<(usize, BoxedStream)>,
    last_index: usize,
}

impl StreamMapAny {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            last_index: 0,
        }
    }

    pub fn insert<S>(&mut self, key: usize, stream: S)
    where
        S: Stream + Unpin + 'static,
        S::Item: Any,
    {
        let boxed = BoxedStream::new(stream);
        self.streams.push((key, boxed));
    }

    pub fn poll_streams(&mut self, cx: &mut Context) -> Poll<Option<(usize, StreamMapAnyVariant)>> {
        let start = self.last_index.wrapping_add(1) % self.streams.len();
        let mut idx = start;
        self.last_index = idx;

        for _ in 0..self.streams.len() {
            let (id, stream) = &mut self.streams[idx];

            match Pin::new(stream).poll_next(cx) {
                Poll::Ready(Some(data)) => {
                    return Poll::Ready(Some((*id, StreamMapAnyVariant(data))));
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

#[derive(Debug)]
pub struct StreamMapAnyVariant(Box<dyn Any>);

impl StreamMapAnyVariant {
    pub fn get_boxed<V>(self: Self) -> Result<Box<V>, Self>
    where
        V: Any,
    {
        self.0.downcast().map_err(Self)
    }

    pub fn get<V>(self: Self) -> Result<V, Self>
    where
        V: Any,
    {
        self.0.downcast().map(|v| *v).map_err(Self)
    }
}

impl Stream for StreamMapAny {
    type Item = (usize, StreamMapAnyVariant);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.streams.is_empty() {
            return Poll::Ready(None);
        }

        self.poll_streams(cx)
    }
}
