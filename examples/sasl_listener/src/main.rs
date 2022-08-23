fn main() {
    println!("Please run example with");
    println!("  cargo run --bin sasl_plain");
    println!("  cargo run --bin sasl_scram_sha_1 --features \"scram\"");
    println!("  cargo run --bin sasl_scram_sha_256 --features \"scram\"");
    println!("  cargo run --bin sasl_scram_sha_512 --features \"scram\"");
}