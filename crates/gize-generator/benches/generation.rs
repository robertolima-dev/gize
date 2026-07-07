//! Benchmarks for the hot generation paths (RC performance tracking).
//!
//! Generation is pure — building a [`Plan`] does no I/O — so these measure the templating and
//! plan assembly the CLI does on `gize new` / `gize make` / `gize sync`. Run with
//! `cargo bench -p gize-generator`. A coarse, non-flaky regression *gate* lives in the test
//! suite (`generation_stays_within_a_generous_time_budget`); these benches are for tracking the
//! actual numbers over time.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gize_core::{Dialect, Manifest, ModelSpec, Module};
use gize_generator::scaffold;

const TS: &str = "20260101000000";

fn product_model() -> ModelSpec {
    ModelSpec::parse(
        "Product",
        &[
            "name:String".to_string(),
            "price:i32".to_string(),
            "active:bool".to_string(),
        ],
    )
    .unwrap()
}

fn manifest_with_modules() -> Manifest {
    let mut m = Manifest::new("shop");
    for name in ["users", "products", "orders", "customers"] {
        m.upsert_module(Module {
            name: name.to_string(),
            fields: vec!["name:String".to_string(), "amount:i64".to_string()],
            belongs_to: Vec::new(),
        });
    }
    m
}

fn bench_new_project(c: &mut Criterion) {
    // The full skeleton with users + OpenAPI + WebSocket — the heaviest `gize new`.
    c.bench_function("new_project (full skeleton)", |b| {
        b.iter(|| {
            scaffold::new_project(
                black_box("shop"),
                true,
                true,
                true,
                Dialect::Postgres,
                None,
                black_box(TS),
            )
        })
    });
}

fn bench_make_crud(c: &mut Criterion) {
    let model = product_model();
    c.bench_function("make_crud (Product)", |b| {
        b.iter(|| scaffold::make_crud(black_box(&model), Dialect::Postgres, black_box(TS)))
    });
}

fn bench_openapi(c: &mut Criterion) {
    let manifest = manifest_with_modules();
    c.bench_function("openapi spec (4 modules)", |b| {
        b.iter(|| scaffold::openapi_json(black_box(&manifest)).unwrap())
    });
}

criterion_group!(benches, bench_new_project, bench_make_crud, bench_openapi);
criterion_main!(benches);
