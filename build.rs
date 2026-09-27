use std::io::Write;

// Build-time deployment config. BRIDGE_API_URL is required (compile fails
// fast otherwise); BRIDGE_TOKEN is optional (empty = open gateway).
// The token is stored XORed with a fresh random salt per build so plain
// `strings` on the binary does not reveal it. Deterrent only: anyone with
// the wasm and this source can recover it. Real isolation is the gateway.
fn main() {
    let api = std::env::var("BRIDGE_API_URL").unwrap_or_default();
    if api.trim().is_empty() {
        panic!("BRIDGE_API_URL must be set at build time (public API base)");
    }
    println!("cargo:rerun-if-env-changed=BRIDGE_API_URL");
    println!("cargo:rerun-if-env-changed=BRIDGE_TOKEN");

    // xorshift64* PRNG seeded from system time: salt only needs uniqueness.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let mut s = now | 1;
    let mut next = move || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s.wrapping_mul(0x2545F4914F6CDD1D)
    };
    let mut salt = [0u8; 16];
    for b in salt.iter_mut() {
        *b = (next() >> 33) as u8;
    }

    let token = std::env::var("BRIDGE_TOKEN").unwrap_or_default();
    let tb = token.as_bytes();
    let mut obf = Vec::with_capacity(tb.len());
    for (i, &b) in tb.iter().enumerate() {
        obf.push(b ^ salt[i % salt.len()]);
    }
    let hex: String = salt.iter().chain(obf.iter()).map(|b| format!("{b:02x}")).collect();
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bridge_cfg.rs");
    let mut f = std::fs::File::create(out).unwrap();
    writeln!(f, "pub const BRIDGE_TOKEN_OBF_HEX: &str = \"{hex}\";").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
