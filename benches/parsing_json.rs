#![allow(unused)]

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crossbeam_queue::ArrayQueue;
use rayon::prelude::*;
use serde_json::Deserializer;

#[path = "../src/rg/de.rs"]
mod de;
#[path = "../src/rg/de_borrow.rs"]
mod de_borrow;

// TIL:
// let nums = (0..10).collect::<Vec<_>>();
// let a = nums.iter().position(|n| *n == 8).unwrap();
// let b = nums.iter().skip(3).position(|n| *n == 8).unwrap();
// assert_ne!(a, b);

const RG_JSON_PATH: &str = "benches/rg.json";

fn bufreader_lines() -> Vec<de::RgMessage> {
    let file = File::open(RG_JSON_PATH).unwrap();
    BufReader::new(file)
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect::<Vec<de::RgMessage>>()
}

fn bufreader_stream() -> Vec<de::RgMessage> {
    let file = File::open(RG_JSON_PATH).unwrap();
    let reader = BufReader::new(file);
    let stream = Deserializer::from_reader(reader);
    stream
        .into_iter()
        .map(|x| x.unwrap())
        .collect::<Vec<de::RgMessage>>()
}

// fastest, but comes at a 2x memory cost
// TODO: are the results out of order?
fn read_to_string_par_bridge() -> Vec<de::RgMessage> {
    std::fs::read_to_string(RG_JSON_PATH)
        .unwrap()
        .lines()
        .par_bridge()
        .map(|x| serde_json::from_str::<de::RgMessage>(&x).unwrap())
        .collect::<Vec<de::RgMessage>>()
}

fn crossbeam_queue() -> Vec<de::RgMessage> {
    let file = File::open(RG_JSON_PATH).unwrap();
    let reader = BufReader::new(file);

    let q = Arc::new(ArrayQueue::new(128));
    let thread_q = q.clone();
    let t = thread::spawn(move || {
        for line in reader.lines() {
            let mut line = line.unwrap();
            loop {
                match thread_q.push(line) {
                    Ok(_) => break,
                    Err(value) => line = value,
                }
            }
        }
    });

    let mut items = vec![];
    while !t.is_finished() {
        while let Some(line) = q.pop() {
            items.push(serde_json::from_str(&line).unwrap());
        }
    }

    items
}

fn divide_and_conquer() -> Vec<de::RgMessage> {
    let file = File::open(RG_JSON_PATH).unwrap();
    // NOTE: this may only be worth it for opening large files; plus, we can't
    // use it when we're parsing `rg`'s stdout anyway...
    let data = unsafe { memmap::MmapOptions::new().map(&file).unwrap() };
    let size = data.len();

    let num_threads = num_cpus::get();
    let chunk_size = size / num_threads;

    thread::scope(|s| {
        let mut results = vec![];
        // TODO: is 0x0a always a newline in utf8? could it be part of a multi-byte code point?
        //  if not, could use std::str::from_utf8_unchecked on `data` and iter chars
        for i in 0..num_threads {
            let start = if i == 0 {
                0
            } else {
                // TODO: handle not found
                let skip = i * chunk_size;
                data.iter().skip(skip).position(|b| *b == 0x0a).unwrap() + skip + 1
            };

            let end = if i == num_threads - 1 {
                size
            } else {
                // TODO: handle not found
                let skip = (i + 1) * chunk_size;
                data.iter().skip(skip).position(|b| *b == 0x0a).unwrap() + skip
            };

            let data = &data[start..end];
            results.push(s.spawn(move || unsafe {
                std::str::from_utf8_unchecked(data)
                    .lines()
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect::<Vec<de::RgMessage>>()
            }));
        }

        // NOTE: these results are out of order (summary not at the end)
        results
            .into_iter()
            .flat_map(|t| t.join().unwrap())
            .collect()
    })
}

fn par_bridge_with_borrow() -> usize {
    let s = std::fs::read_to_string(RG_JSON_PATH).unwrap();
    let items = black_box(
        s.lines()
            .par_bridge()
            .map(|x| serde_json::from_str(&x).unwrap())
            .collect::<Vec<de_borrow::RgMessage>>(),
    );

    items.len()
}

fn par_bridge_mmap_with_borrow() -> usize {
    let file = File::open(RG_JSON_PATH).unwrap();
    // NOTE: this may only be worth it for opening large files; plus, we can't
    // use it when we're parsing `rg`'s stdout anyway...
    let data = unsafe { memmap::MmapOptions::new().map(&file).unwrap() };
    let s = unsafe { std::str::from_utf8_unchecked(&data[..]) };
    let items = black_box(
        s.lines()
            .par_bridge()
            .map(|x| serde_json::from_str(x).unwrap())
            .collect::<Vec<de_borrow::RgMessage>>(),
    );

    items.len()
}

fn divide_and_conquer_with_borrow() -> usize {
    let file = File::open(RG_JSON_PATH).unwrap();
    // NOTE: this may only be worth it for opening large files; plus, we can't
    // use it when we're parsing `rg`'s stdout anyway...
    let data = unsafe { memmap::MmapOptions::new().map(&file).unwrap() };
    let size = data.len();

    let num_threads = num_cpus::get();
    let chunk_size = size / num_threads;

    thread::scope(|s| {
        let mut results = vec![];
        // TODO: is 0x0a always a newline in utf8? could it be part of a multi-byte code point?
        //  if not, could use std::str::from_utf8_unchecked on `data` and iter chars
        for i in 0..num_threads {
            let start = if i == 0 {
                0
            } else {
                // TODO: handle not found
                let skip = i * chunk_size;
                data.iter().skip(skip).position(|b| *b == 0x0a).unwrap() + skip + 1
            };

            let end = if i == num_threads - 1 {
                size
            } else {
                // TODO: handle not found
                let skip = (i + 1) * chunk_size;
                data.iter().skip(skip).position(|b| *b == 0x0a).unwrap() + skip
            };

            let data = &data[start..end];
            results.push(s.spawn(move || unsafe {
                std::str::from_utf8_unchecked(data)
                    .lines()
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect::<Vec<de_borrow::RgMessage>>()
            }));
        }

        // NOTE: these results are out of order (summary not at the end)
        let items: Vec<de_borrow::RgMessage> = black_box(
            results
                .into_iter()
                .flat_map(|t| t.join().unwrap())
                .collect(),
        );

        items.len()
    })
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("parsing json");
    g.measurement_time(Duration::from_secs(20));

    // these borrow the data
    g.bench_function("0 read_to_string().lines().par_bridge() [borrow]", |b| {
        b.iter(|| par_bridge_with_borrow())
    });
    g.bench_function("1 mmap & thread parse [borrow]", |b| {
        b.iter(|| divide_and_conquer_with_borrow())
    });
    g.bench_function(
        "2 read_to_string().lines().par_bridge() [borrow+mmap]",
        |b| b.iter(|| par_bridge_mmap_with_borrow()),
    );
    // these don't take up more memory than they need
    g.bench_function("3 BufReader::lines", |b| b.iter(|| bufreader_lines()));
    g.bench_function("4 StreamDeserializer", |b| b.iter(|| bufreader_stream()));
    g.bench_function("5 BufReader::lines + ArrayQueue", |b| {
        b.iter(|| crossbeam_queue())
    });
    // these take twice the memory needed
    g.bench_function("6 mmap & thread parse", |b| b.iter(|| divide_and_conquer()));
    g.bench_function("7 read_to_string().lines().par_bridge()", |b| {
        b.iter(|| read_to_string_par_bridge())
    });

    g.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
