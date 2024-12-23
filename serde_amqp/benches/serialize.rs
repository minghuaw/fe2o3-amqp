#![allow(clippy::all)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{
    distributions::{Alphanumeric, DistString},
    Rng, RngCore,
};
use serde_amqp::primitives::{Binary, Dec128, Dec32, Dec64, Timestamp};

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

    // 16 bytes of u64
    let mut value = vec![0u64; 16 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 64 bytes of u64
    let mut value = vec![0u64; 64 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 64B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 256 bytes of u64
    let mut value = vec![0u64; 256 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 256B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 1kB of u64
    let mut value = vec![0u64; 1024 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 1MB of u64
    let mut value = vec![0u64; 1024 * 1024 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 1MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 10MB of u64
    let mut value = vec![0u64; 10 * 1024 * 1024 / std::mem::size_of::<u64>()];
    rand::thread_rng().fill(&mut value[..]);
    c.bench_function("serialize List<u64> 10MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // list of strings
    // 10 strings of 16B
    let value = (0..10)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 16))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 10x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 100 strings of 16B
    let value = (0..100)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 16))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 100x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 1000 strings of 16B
    let value = (0..1000)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 16))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 1000x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 10 strings of 1kB
    let value = (0..10)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 1024))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 10x1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 100 strings of 1kB
    let value = (0..100)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 1024))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 100x1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 1000 strings of 1kB
    let value = (0..1000)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 1024))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 1000x1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 10 strings of 1MB
    let value = (0..10)
        .map(|_| Alphanumeric.sample_string(&mut rand::thread_rng(), 1024 * 1024))
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 10x1MB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 10 Random strings of size between 16B and 1kB
    let value = (0..10)
        .map(|_| {
            let size = rand::thread_rng().gen_range(16..1024);
            Alphanumeric.sample_string(&mut rand::thread_rng(), size)
        })
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 10x16B-10kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 100 Random strings of size between 16B and 1kB
    let value = (0..100)
        .map(|_| {
            let size = rand::thread_rng().gen_range(16..1024);
            Alphanumeric.sample_string(&mut rand::thread_rng(), size)
        })
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 100x16B-10kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // 1000 Random strings of size between 16B and 1kB
    let value = (0..1000)
        .map(|_| {
            let size = rand::thread_rng().gen_range(16..1024);
            Alphanumeric.sample_string(&mut rand::thread_rng(), size)
        })
        .map(String::from)
        .collect::<Vec<_>>();
    c.bench_function("serialize List<String> 1000x16B-10kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 10 u64 -> u64
    let value = (0..10)
        .map(|_| {
            let key = rand::random::<u64>();
            let value = rand::random::<u64>();
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<u64, u64> 10", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 100 u64 -> u64
    let value = (0..100)
        .map(|_| {
            let key = rand::random::<u64>();
            let value = rand::random::<u64>();
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<u64, u64> 100", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 1000 u64 -> u64
    let value = (0..1000)
        .map(|_| {
            let key = rand::random::<u64>();
            let value = rand::random::<u64>();
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<u64, u64> 1000", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 10 16B String -> 16B String
    let value = (0..10)
        .map(|_| {
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let key = String::from(key);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 10x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 100 16B String -> 16B String
    let value = (0..100)
        .map(|_| {
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let key = String::from(key);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 100x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 1000 16B String -> 16B String
    let value = (0..1000)
        .map(|_| {
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let key = String::from(key);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 1000x16B", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 10 random String (16B-1kB) -> random String (16B-1kB)
    let value = (0..10)
        .map(|_| {
            let key_size = rand::thread_rng().gen_range(16..1024);
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), key_size);
            let key = String::from(key);
            let value_size = rand::thread_rng().gen_range(16..1024);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), value_size);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 10x16B-1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 100 random String (16B-1kB) -> random String (16B-1kB)
    let value = (0..100)
        .map(|_| {
            let key_size = rand::thread_rng().gen_range(16..1024);
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), key_size);
            let key = String::from(key);
            let value_size = rand::thread_rng().gen_range(16..1024);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), value_size);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 100x16B-1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });

    // Map of 1000 random String (16B-1kB) -> random String (16B-1kB)
    let value = (0..1000)
        .map(|_| {
            let key_size = rand::thread_rng().gen_range(16..1024);
            let key = Alphanumeric.sample_string(&mut rand::thread_rng(), key_size);
            let key = String::from(key);
            let value_size = rand::thread_rng().gen_range(16..1024);
            let value = Alphanumeric.sample_string(&mut rand::thread_rng(), value_size);
            let value = String::from(value);
            (key, value)
        })
        .collect::<std::collections::HashMap<_, _>>();
    c.bench_function("serialize Map<String, String> 1000x16B-1kB", |b| {
        b.iter(|| serde_amqp::to_vec(black_box(&value)).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
