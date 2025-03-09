use circuit::proof::{generate_gkr_proof, verify_gkr_proof, GKRProver};
use circuit::scenarios::{self, create_large_batch_test};
use expander_transcript::Proof;
use std::time::Instant;
use std::fs::File;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    // Use create_large_batch_test from scenarios.rs for a realistic batch
    let (circuits, previous_proofs) = create_large_batch_test();
    let circuits_refs: Vec<&dyn GKRProver> = circuits.iter().map(|boxed| boxed.as_ref()).collect();

    // Generate proofs
    let start = Instant::now();
    let (recursive_proof, all_proofs) = generate_gkr_proof(&circuits_refs, &previous_proofs);
    let gen_time = start.elapsed();

    let verify_start = Instant::now();
    let valid = verify_gkr_proof(&recursive_proof, &all_proofs);
    let verify_time = verify_start.elapsed();

    // Print raw 32-byte proofs as hex
    println!("Recursive Proof (32 bytes): 0x{}", hex::encode(&recursive_proof.bytes));
    println!("Individual Proofs ({} × 32 bytes):", all_proofs.len());
    for (i, proof) in all_proofs.iter().enumerate() {
        println!("  Proof {}: 0x{}", i, hex::encode(&proof.bytes));
    }

    // Collect public outputs (amount0, amount1) from circuits
    let public_outputs: Vec<(String, String)> = circuits.iter()
        .map(|circuit| {
            let outputs = circuit.get_public_outputs();
            let mut amount0_bytes = [0u8; 32];
            let mut amount1_bytes = [0u8; 32];
            outputs[0].to_little_endian(&mut amount0_bytes); 
            outputs[1].to_little_endian(&mut amount1_bytes);
            (format!("0x{}", hex::encode(amount0_bytes)), format!("0x{}", hex::encode(amount1_bytes)))
        })
        .collect();

    // Write public outputs to file
    let mut file = File::create("proofs.txt")?;
    writeln!(file, "Recursive Proof: 0x{}", hex::encode(&recursive_proof.bytes))?;
    writeln!(file, "Number of Individual Proofs: {}", all_proofs.len())?;
    for (i, proof) in all_proofs.iter().enumerate() {
        writeln!(file, "Proof {}: 0x{}", i, hex::encode(&proof.bytes))?;
    }
    writeln!(file, "Public Outputs:")?;
    for (i, (amount0, amount1)) in public_outputs.iter().enumerate() {
        writeln!(file, "Circuit {} - Amount0: {}, Amount1: {}", i, amount0, amount1)?;
    }

    // Summary table
    let results = vec![TestResult {
        name: "Large Batch Test".to_string(),
        num_circuits: circuits.len(),
        gen_time_ms: gen_time.as_micros() as f64 / 1000.0,
        verify_time_ms: verify_time.as_micros() as f64 / 1000.0,
        valid,
    }];
    print_summary_table(&results);

    if valid && all_proofs.len() == circuits.len() {
        println!("All tests passed successfully!");
        println!("Proofs written to proofs.txt");
        Ok(()) // Return Ok(()) for successful case
    } else {
        // Return an error instead of panicking
        Err(io::Error::new(io::ErrorKind::Other, "Verification failed or proof count mismatch"))
    }
}

struct TestResult {
    name: String,
    num_circuits: usize,
    gen_time_ms: f64,
    verify_time_ms: f64,
    valid: bool,
}

fn print_summary_table(results: &[TestResult]) {
    println!("===== Test Summary =====");
    println!("{:-<80}", "");
    println!(
        "| {:<20} | {:<12} | {:<18} | {:<18} | {:<8} |",
        "Test Name", "Circuits", "Gen Time (ms)", "Verify Time (ms)", "Valid"
    );
    println!("{:-<80}", "");
    for result in results {
        println!(
            "| {:<20} | {:<12} | {:<18.2} | {:<18.2} | {:<8} |",
            result.name,
            result.num_circuits,
            result.gen_time_ms,
            result.verify_time_ms,
            if result.valid { "Yes" } else { "No" }
        );
    }
    println!("{:-<80}", "");
}