typedflake::id!(UserId);
typedflake::id!(OrderId);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read from environment (K8s, Docker, etc.)
    let worker_id = std::env::var("POD_ORDINAL")
        .unwrap_or_else(|_| "1".into())
        .parse()?;
    let process_id = std::env::var("CONTAINER_ID")
        .unwrap_or_else(|_| "2".into())
        .parse()?;

    // Set default instance once at startup
    typedflake::global::set_default_instance(worker_id, process_id)?;

    // All subsequent ID generation uses default config
    let user_id = UserId::generate(); // Uses default config + instance

    println!("User ID: {}", user_id.timestamp());
    Ok(())
}
