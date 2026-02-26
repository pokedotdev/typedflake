use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(TypedFlake)]
pub struct OrderId(u64);

#[derive(TypedFlake)]
#[typedflake(layout = (42, 8, 4, 10), epoch = "2025-01-01")]
pub struct SessionId(u64);

fn main() {
    // Basic generation (default instance: worker=0, process=0)
    let user = UserId::generate();
    let order = OrderId::generate();
    let session = SessionId::generate();

    println!("Generated IDs:");
    println!("  User:    {user}");
    println!("  Order:   {order}");
    println!("  Session: {session}");

    // Instance-based generation for distributed systems
    let worker_5 = UserId::worker(5).unwrap();
    let id1 = worker_5.generate();
    let id2 = worker_5.generate();

    println!("\nWorker 5 IDs:");
    println!("  ID 1: {id1}");
    println!("  ID 2: {id2}");

    // Extract components
    let (timestamp, worker_id, process_id, sequence) = user.decompose();
    println!("\nUser ID components:");
    println!("  Timestamp:  {timestamp}");
    println!("  Worker ID:  {worker_id}");
    println!("  Process ID: {process_id}");
    println!("  Sequence:   {sequence}");
}
