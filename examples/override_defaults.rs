use typedflake::Config;

// Discord's configuration
const DISCORD_CONFIG: Config = Config::new(
    (42, 5, 5, 12),
    1_420_070_400_000, // The first second of 2015
);

// Define ID types at module scope
typedflake::id!(UserId);
typedflake::id!(ServerId);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read from environment (K8s, Docker, etc.)
    let worker_id = std::env::var("POD_ORDINAL")
        .unwrap_or_else(|_| "1".into())
        .parse()?;
    let process_id = std::env::var("CONTAINER_ID")
        .unwrap_or_else(|_| "2".into())
        .parse()?;

    // Set default configuration once at startup
    typedflake::global::set_defaults(DISCORD_CONFIG, worker_id, process_id)?;

    // All subsequent ID generation uses default config
    let user_id = UserId::generate()?;
    let server_id = ServerId::generate()?;

    println!("User ID:  {user_id}");
    println!("Server ID: {server_id}");

    // Show that they use the default instance
    println!(
        "Worker: {}, Process: {}",
        user_id.worker_id(),
        user_id.process_id()
    );

    Ok(())
}
