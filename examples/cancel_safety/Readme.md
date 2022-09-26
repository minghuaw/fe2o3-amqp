# Cancellation safety workaround

The current impelmentation of `Receiver::recv()` does not guarantee cancellation safety and thus may lead to data loss if `Receiver::recv()` is put inside a `select!` block.

This example aims to demonstrate a workaround while cancellation safety issue is being resolved.
