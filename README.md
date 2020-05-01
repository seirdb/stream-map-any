Allows merging async Streams of different output type.

It's very similar to Tokio's `StreamMap`, except that it doesn't require the streams to have the
same output type.
This can be usefull when you don't know what type of streams should be combined, acting as a
runtime dynamic select.

# Not a zero-cost-abstraction
Since we don't know what types of outputs the streams will generate, the generated output will
be a `StreamMapAnyVariant`, a newtype around `Box<dyn Any>`. As a result, we rely on dynamic
dispatching to transform it back into the desired output.
Benching shows that it's __2x__ as slow as a `StreamMap` or Tokio's `select` macro.
