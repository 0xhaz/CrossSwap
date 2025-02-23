use criterion::{black_box, criterion_group, criterion_main, Criterion};
use circuit::proof::{generate_gkr_proof, verify_gkr_proof};
use expander_transcript::Proof;

fn bench_gkr_proof_generation(c: &mut Criterion) {
    let proofs = vec![];
    c.bench_function("gkr_proof_generation", |b| {
        b.iter(|| generate_gkr_proof(black_box(&proofs)))
    });
}

fn bench_gkr_proof_verification(c: &mut Criterion) {
    let proofs = vec![];
    let proof = generate_gkr_proof(&proofs);
    c.bench_function("gkr_proof_verification", |b| {
        b.iter(|| verify_gkr_proof(black_box(&proof), black_box(&proofs)))
    });
}

criterion_group!(benches, bench_gkr_proof_generation, bench_gkr_proof_verification);
criterion_main!(benches);