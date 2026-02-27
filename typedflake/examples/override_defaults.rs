use typedflake::{BitLayout, Config, Epoch, TypedFlake};

const CUSTOM_CONFIG: Config =
    Config::new(BitLayout::new(42, 5, 5, 12), Epoch::from_date(2025, 1, 1));

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(TypedFlake)]
pub struct ServerId(u64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read from environment (K8s, Docker, etc.)
    let worker_id = std::env::var("POD_ORDINAL")
        .unwrap_or_else(|_| "1".into())
        .parse()?;
    let process_id = std::env::var("CONTAINER_ID")
        .unwrap_or_else(|_| "2".into())
        .parse()?;

    // Set default configuration once at startup
    typedflake::defaults()
        .config(CUSTOM_CONFIG)
        .instance(worker_id, process_id)
        .init()?;

    // All subsequent ID generation uses default config
    let user_id = UserId::generate();
    let server_id = ServerId::generate();

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
