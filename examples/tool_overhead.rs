use serde_json::json;
use std::time::Instant;
fn main() {
    let ctx = perci::tools::Ctx::new();
    let tools = perci::tools::catalog();
    let exact = perci::tools::resolve(&tools, "exact").unwrap();
    let args = json!({"expr": "3/7 + 2/9"});
    for _ in 0..100 {
        perci::tools::run(&ctx, exact.as_ref(), &args);
    }
    let n = 2000;
    let t = Instant::now();
    for _ in 0..n {
        let _ = exact.call(&ctx, &args);
    }
    println!(
        "call only: {:.1} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let t = Instant::now();
    for _ in 0..n {
        let _ = perci::tools::run(&ctx, exact.as_ref(), &args);
    }
    println!(
        "run+seal: {:.1} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let v = perci::tools::run(&ctx, exact.as_ref(), &args);
    let t = Instant::now();
    for _ in 0..n {
        let _ = v.to_value().to_string();
    }
    println!(
        "to_value+string: {:.1} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let t = Instant::now();
    for _ in 0..n {
        let _ = perci::verdict::sha256_canonical(&v.result);
    }
    println!(
        "sha canonical: {:.1} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let t = Instant::now();
    for _ in 0..n {
        let _ = perci::branding::build_id();
    }
    println!(
        "build_id: {:.1} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
}
