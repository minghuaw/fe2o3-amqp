use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{RngCore, distributions::{Alphanumeric, DistString}};
use serde_amqp::primitives::{Dec32, Dec64, Dec128, Timestamp, Binary};

fn criterion_benchmark(c: &mut Criterion) {
    let value = ();
    c.bench_function("serialize null", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<bool>();
    c.bench_function("serialize bool", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<u8>();
    c.bench_function("serialize u8", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<u16>();
    c.bench_function("serialize u16", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<u32>();
    c.bench_function("serialize u32", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<u64>();
    c.bench_function("serialize u64", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<i8>();
    c.bench_function("serialize i8", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<i16>();
    c.bench_function("serialize i16", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<i32>();
    c.bench_function("serialize i32", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<i64>();
    c.bench_function("serialize i64", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<f32>();
    c.bench_function("serialize f32", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<f64>();
    c.bench_function("serialize f64", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Dec32::from(rand::random::<[u8; 4]>());
    c.bench_function("serialize Dec32", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Dec64::from(rand::random::<[u8; 8]>());
    c.bench_function("serialize Dec64", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Dec128::from(rand::random::<[u8; 16]>());
    c.bench_function("serialize Dec128", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = rand::random::<char>();
    c.bench_function("serialize char", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let timestamp = Timestamp::from_milliseconds(rand::random::<i64>());
    c.bench_function("serialize Timestamp", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&timestamp)).unwrap())
    });

    let uuid = uuid::Uuid::new_v4();
    let uuid = serde_amqp::primitives::Uuid::from(uuid.to_bytes_le());
    c.bench_function("serialize Uuid", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&uuid)).unwrap())
    });

    let mut value = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let mut value = vec![0u8; 64];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 64B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let mut value = vec![0u8; 256];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 256B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let mut value = vec![0u8; 1024];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let mut value = vec![0u8; 1024 * 1024];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 1MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let mut value = vec![0u8; 10 * 1024 * 1024];
    rand::thread_rng().fill_bytes(&mut value);
    let value = Binary::from(value);
    c.bench_function("serialize Binary 10MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let value = String::from(value);
    c.bench_function("serialize String 16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);
    let value = String::from(value);
    c.bench_function("serialize String 64B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 256);
    let value = String::from(value);
    c.bench_function("serialize String 256B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 1024);
    let value = String::from(value);
    c.bench_function("serialize String 1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 1024 * 1024);
    let value = String::from(value);
    c.bench_function("serialize String 1MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 10 * 1024 * 1024);
    let value = String::from(value);
    c.bench_function("serialize String 10MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Symbol is very similar to String, so we don't benchmark it.

    // TODO: How to bench list, map, and array? Define number of items or bytes?
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
