use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tinyolap::encoding::{Codec, Primitive, StringCodec};

const N: usize = 1_000_000;

fn to_raw<T: Primitive>(data: &[T]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * T::WIDTH);
    for &v in data {
        v.encode_le(&mut out);
    }
    out
}

fn bench_plain(c: &mut Criterion) {
    let data: Vec<i64> = (0..N as i64).collect();
    let raw = to_raw(&data);
    let bytes = raw.len() as u64;

    let mut encoded = Vec::new();
    Codec::Plain.encode(&raw, i64::WIDTH, &mut encoded);

    let mut group = c.benchmark_group("codec_plain");
    group.throughput(Throughput::Bytes(bytes));

    group.bench_function("encode_i64_1M", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(raw.len());
            Codec::Plain.encode(black_box(&raw), i64::WIDTH, &mut out);
            black_box(out);
        });
    });

    group.bench_function("decode_i64_1M", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(encoded.len());
            Codec::Plain.decode(black_box(&encoded), i64::WIDTH, &mut out).unwrap();
            black_box(out);
        });
    });

    group.finish();
}

fn bench_delta(c: &mut Criterion) {
    let sorted: Vec<i64> = (0..N as i64).collect();
    let random: Vec<i64> = (0..N as i64)
        .map(|mut x| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x })
        .collect();

    let bytes = (N * i64::WIDTH) as u64;

    let mut group = c.benchmark_group("codec_delta");
    group.throughput(Throughput::Bytes(bytes));

    for (label, data) in [("sorted", &sorted), ("random", &random)] {
        let raw = to_raw(data);

        group.bench_with_input(
            BenchmarkId::new("encode_i64_1M", label),
            &raw,
            |b, raw| {
                b.iter(|| {
                    let mut out = Vec::with_capacity(raw.len());
                    Codec::Delta.encode(black_box(raw), i64::WIDTH, &mut out);
                    black_box(out);
                });
            },
        );

        let mut encoded = Vec::new();
        Codec::Delta.encode(&raw, i64::WIDTH, &mut encoded);

        group.bench_with_input(
            BenchmarkId::new("decode_i64_1M", label),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let mut out = Vec::with_capacity(encoded.len());
                    Codec::Delta.decode(black_box(encoded), i64::WIDTH, &mut out).unwrap();
                    black_box(out);
                });
            },
        );
    }

    group.finish();
}

fn bench_rle(c: &mut Criterion) {
    let high_run: Vec<i64>  = vec![42; N];
    let low_card: Vec<i64>  = (0..N as i64).map(|i| i % 10).collect();
    let all_unique: Vec<i64> = (0..N as i64).collect();

    let bytes = (N * i64::WIDTH) as u64;

    let mut group = c.benchmark_group("codec_rle");
    group.throughput(Throughput::Bytes(bytes));

    for (label, data) in [
        ("high_run", &high_run),
        ("low_cardinality", &low_card),
        ("all_unique", &all_unique),
    ] {
        let raw = to_raw(data);

        group.bench_with_input(
            BenchmarkId::new("encode_i64_1M", label),
            &raw,
            |b, raw| {
                b.iter(|| {
                    let mut out = Vec::new();
                    Codec::RLE.encode(black_box(raw), i64::WIDTH, &mut out);
                    black_box(out);
                });
            },
        );

        let mut encoded = Vec::new();
        Codec::RLE.encode(&raw, i64::WIDTH, &mut encoded);

        group.bench_with_input(
            BenchmarkId::new("decode_i64_1M", label),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let mut out = Vec::new();
                    Codec::RLE.decode(black_box(encoded), i64::WIDTH, &mut out).unwrap();
                    black_box(out);
                });
            },
        );
    }

    group.finish();
}

fn bench_string_codecs(c: &mut Criterion) {
    const EVENT_NAMES: &[&str] = &[
        "click", "view", "purchase", "scroll", "hover",
        "login", "logout", "search", "share", "bookmark",
    ];
    let data: Vec<String> = (0..N)
        .map(|i| EVENT_NAMES[i % EVENT_NAMES.len()].to_string())
        .collect();
    let bytes: u64 = data.iter().map(|s| s.len() as u64).sum();

    let mut group = c.benchmark_group("codec_string");
    group.throughput(Throughput::Bytes(bytes));

    for &codec in &[StringCodec::Plain, StringCodec::Dictionary] {
        let label = match codec {
            StringCodec::Plain      => "plain",
            StringCodec::Dictionary => "dictionary",
        };

        group.bench_function(BenchmarkId::new("encode_1M", label), |b| {
            b.iter(|| {
                let mut out = Vec::new();
                codec.encode(black_box(&data), &mut out);
                black_box(out);
            });
        });

        let mut encoded = Vec::new();
        codec.encode(&data, &mut encoded);

        group.bench_function(BenchmarkId::new("decode_1M", label), |b| {
            b.iter(|| {
                let mut out = Vec::new();
                codec.decode(black_box(&encoded), &mut out).unwrap();
                black_box(out);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_plain, bench_delta, bench_rle, bench_string_codecs);
criterion_main!(benches);
