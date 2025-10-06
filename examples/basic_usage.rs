use typedflake::{BitLayout, Config, Epoch};

// Create ID types with default configuration
typedflake::id!(UserId);
typedflake::id!(OrderId);

// Create ID type with custom configuration
const CUSTOM_CONFIG: Config = Config::new_unchecked(
    BitLayout::new(42, 5, 5, 12),
    Epoch::from_date(2025, 1, 1), // January 1, 2025
);

typedflake::id!(SessionId, CUSTOM_CONFIG);

fn main() {
    println!("=== TypedFlake Demo ===\n");

    // Generate some IDs with default instance (0, 0)
    println!("1. Basic ID Generation (default instance):");
    let user_id = UserId::generate().unwrap();
    let order_id = OrderId::generate().unwrap();
    let session_id = SessionId::generate().unwrap();

    println!("User ID:    {user_id}");
    println!("Order ID:   {order_id}");
    println!("Session ID: {session_id}");

    // Generate IDs with specific instances
    println!("\n2. Instance-based ID Generation:");
    let worker_instance = UserId::worker(15).unwrap();
    let process_instance = UserId::process(7).unwrap();
    let full_instance = UserId::instance(31, 3).unwrap();

    let worker_id = worker_instance.generate().unwrap();
    let process_id = process_instance.generate().unwrap();
    let full_id = full_instance.generate().unwrap();

    println!("Worker instance ID (15, 0):  {worker_id}");
    println!("Process instance ID (0, 7):  {process_id}");
    println!("Full instance ID (31, 3):    {full_id}");

    // Show components
    println!("\n3. ID Components:");
    let user_components = user_id.components();
    println!("User ID components (default 0,0):");
    println!("  Timestamp: {}", user_components.timestamp);
    println!("  Worker ID: {}", user_components.worker_id);
    println!("  Process ID: {}", user_components.process_id);
    println!("  Sequence: {}", user_components.sequence);

    let full_components = full_id.components();
    println!("\nFull instance ID components (31,3):");
    println!("  Timestamp: {}", full_components.timestamp);
    println!("  Worker ID: {}", full_components.worker_id);
    println!("  Process ID: {}", full_components.process_id);
    println!("  Sequence: {}", full_components.sequence);

    // Show individual access methods
    println!("\n4. Individual Component Access:");
    println!("User ID timestamp: {}", user_id.timestamp());
    println!("User ID sequence:  {}", user_id.sequence());

    // Show decomposition
    println!("\n5. Decomposition:");
    let (timestamp, worker_id, process_id, sequence) = user_id.decompose();
    println!(
        "User ID decomposed: timestamp={timestamp}, worker_id={worker_id}, process_id={process_id}, sequence={sequence}"
    );

    // Show composition
    println!("\n6. Composition:");
    let composed_id = UserId::compose_custom(timestamp, worker_id, process_id, sequence).unwrap();
    println!("Recomposed User ID: {composed_id}");
    println!(
        "Original equals recomposed: {}",
        user_id.as_u64() == composed_id.as_u64()
    );

    // Show conversions
    println!("\n7. Conversions:");
    let raw_value = user_id.as_u64();
    let from_raw = UserId::from_u64_unchecked(raw_value);
    println!("Raw value: {raw_value}");
    println!("From raw:  {from_raw}");
    println!("Equal to original: {}", user_id == from_raw);

    // Show parsing
    println!("\n8. String Parsing:");
    let id_string = user_id.to_string();
    let parsed_id: UserId = id_string.parse().unwrap();
    println!("ID as string: {id_string}");
    println!("Parsed back:  {parsed_id}");
    println!("Equal to original: {}", user_id == parsed_id);

    // Generate multiple IDs quickly with different instances
    println!("\n9. Multiple ID Generation with Threading Simulation:");
    for thread_id in 0..5 {
        let instance = UserId::process(thread_id).unwrap();
        let id = instance.generate_blocking();
        println!("Thread {thread_id}:");
        println!("ID: {id}");
        let (timestamp, worker_id, process_id, sequence) = id.decompose();
        println!(
            "Timestamp: {timestamp} | Worker ID: {worker_id} | Process ID: {process_id} | Sequence: {sequence}",
        );
    }

    // Show validation from external sources
    println!("\n10. Validation (Newtype Safety):");

    // Simulating ID from database/API
    let db_value = user_id.as_u64();

    // ✅ Validated construction (preferred for external data)
    match UserId::try_from_u64(db_value) {
        Ok(validated_id) => println!("Valid ID from database: {validated_id}"),
        Err(e) => println!("Invalid ID from database: {e}"),
    }

    // ✅ TryFrom trait
    let converted: Result<UserId, _> = db_value.try_into();
    println!("TryFrom conversion: {}", converted.is_ok());

    // ⚠️ Unchecked construction (performance-critical only)
    let unchecked_id = UserId::from_u64_unchecked(db_value);
    println!("Unchecked ID (trusted source): {unchecked_id}");

    // Show validation errors
    println!("\n11. Validation Errors:");
    let (ts, _w, p, s) = user_id.decompose();

    // This will fail validation (worker_id out of range)
    match UserId::compose_custom(ts, 999, p, s) {
        Ok(_) => println!("Composed successfully"),
        Err(e) => println!("Composition failed: {e}"),
    }

    // This will silently mask (unchecked)
    let masked_id = UserId::compose_custom_unchecked(ts, 999, p, s);
    println!(
        "Unchecked composition masks overflow: worker_id={}",
        masked_id.worker_id()
    );

    println!("\n=== Demo Complete ===");
}
