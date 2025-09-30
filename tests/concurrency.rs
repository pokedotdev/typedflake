// ==================== Concurrent Test Helpers ====================

/// Run concurrent ID generation test with multiple threads
/// Returns Vec<Vec<T>> where each inner Vec contains IDs from one thread
fn run_concurrent_generation<T, F>(
    num_threads: usize,
    _ids_per_thread: usize,
    generator_fn: F,
) -> Vec<Vec<T>>
where
    T: Send + 'static,
    F: Fn(usize) -> Vec<T> + Send + Sync + Clone + 'static,
{
    use std::thread;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_idx| {
            let generator = generator_fn.clone();
            thread::spawn(move || generator(thread_idx))
        })
        .collect();

    handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect()
}

/// Assert that all IDs in a collection are unique
fn assert_all_unique<T: Ord + Clone + std::fmt::Debug>(ids: &[T], context: &str) {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let original_len = sorted.len();
    sorted.dedup();
    assert_eq!(
        original_len,
        sorted.len(),
        "{}: Found {} duplicate IDs out of {}",
        context,
        original_len - sorted.len(),
        original_len
    );
}

/// Assert global uniqueness across multiple thread results
fn assert_global_uniqueness<T: Ord + Clone + std::fmt::Debug>(
    thread_results: Vec<Vec<T>>,
    context: &str,
) {
    let mut all_ids: Vec<T> = thread_results.into_iter().flatten().collect();
    let total = all_ids.len();
    all_ids.sort();
    all_ids.dedup();
    assert_eq!(
        total,
        all_ids.len(),
        "{}: Found {} duplicate IDs across threads (total {})",
        context,
        total - all_ids.len(),
        total
    );
}

/// Helper to verify component values match expected worker/process IDs
fn assert_components_match<T>(
    ids: &[T],
    expected_worker: u64,
    expected_process: u64,
    components_fn: impl Fn(&T) -> (u64, u64),
) {
    for id in ids {
        let (worker_id, process_id) = components_fn(id);
        assert_eq!(worker_id, expected_worker, "Worker ID mismatch");
        assert_eq!(process_id, expected_process, "Process ID mismatch");
    }
}

// ==================== Tests ====================

#[test]
fn lock_free_stress_8_threads() {
    typedflake::id!(StressTestId);

    const NUM_THREADS: usize = 8;
    const IDS_PER_THREAD: usize = 1000;

    // Run concurrent ID generation with different worker IDs per thread
    let thread_results = run_concurrent_generation(NUM_THREADS, IDS_PER_THREAD, |thread_id| {
        let instance = StressTestId::instance(thread_id as u64, 0).unwrap();
        let mut ids = Vec::with_capacity(IDS_PER_THREAD);

        for _ in 0..IDS_PER_THREAD {
            ids.push(instance.generate_blocking());
        }

        // Verify thread-local uniqueness
        assert_all_unique(&ids, &format!("Thread {thread_id} local uniqueness"));
        ids
    });

    // Verify global uniqueness across all threads
    assert_global_uniqueness(thread_results, "8 threads × 1000 IDs");
}

#[test]
fn state_isolation_per_instance() {
    typedflake::id!(IsolationTestId);

    const NUM_INSTANCES: usize = 4;
    const IDS_PER_INSTANCE: usize = 100;

    // Test that different (worker_id, process_id) pairs have independent sequences
    let results = run_concurrent_generation(NUM_INSTANCES, IDS_PER_INSTANCE, |i| {
        let worker_id = i as u64;
        let process_id = (i * 2) as u64;
        let instance = IsolationTestId::instance(worker_id, process_id).unwrap();

        let mut ids = Vec::with_capacity(IDS_PER_INSTANCE);
        for _ in 0..IDS_PER_INSTANCE {
            ids.push(instance.generate_blocking());
        }

        // Verify IDs have correct worker/process components
        assert_components_match(&ids, worker_id, process_id, |id| {
            let c = id.components();
            (c.worker_id, c.process_id)
        });

        // Extract sequences for verification
        let sequences: Vec<u64> = ids.iter().map(|id| id.sequence()).collect();

        // Each instance should start from sequence 0
        assert_eq!(
            sequences[0], 0,
            "Instance ({worker_id}, {process_id}) should start at 0"
        );

        // Sequences should show reasonable progression
        let max_sequence = *sequences.iter().max().unwrap();
        assert!(
            max_sequence >= IDS_PER_INSTANCE as u64 / 2,
            "Instance ({worker_id}, {process_id}) should have reasonable progression"
        );

        ids
    });

    // Verify global uniqueness
    assert_global_uniqueness(results, "Per-instance state isolation");
}

#[test]
fn concurrent_min_boundary() {
    // Test same instance (0, 0) from multiple threads - minimum boundary
    typedflake::id!(MinInstanceId);

    const NUM_THREADS: usize = 4;
    const IDS_PER_THREAD: usize = 250;
    const WORKER_ID: u64 = 0;
    const PROCESS_ID: u64 = 0;

    let thread_results = run_concurrent_generation(NUM_THREADS, IDS_PER_THREAD, |_| {
        let instance = MinInstanceId::instance(WORKER_ID, PROCESS_ID).unwrap();
        (0..IDS_PER_THREAD)
            .map(|_| instance.generate_blocking())
            .collect()
    });

    let all_ids: Vec<_> = thread_results.into_iter().flatten().collect();
    assert_all_unique(&all_ids, "Concurrent min instance (0,0)");
    assert_components_match(&all_ids, WORKER_ID, PROCESS_ID, |id| {
        let c = id.components();
        (c.worker_id, c.process_id)
    });
}

#[test]
fn concurrent_max_boundary() {
    // Test same instance (31, 31) from multiple threads - maximum boundary
    typedflake::id!(MaxInstanceId);

    const NUM_THREADS: usize = 4;
    const IDS_PER_THREAD: usize = 250;
    const WORKER_ID: u64 = 31; // Max for 5 bits
    const PROCESS_ID: u64 = 31; // Max for 5 bits

    let thread_results = run_concurrent_generation(NUM_THREADS, IDS_PER_THREAD, |_| {
        let instance = MaxInstanceId::instance(WORKER_ID, PROCESS_ID).unwrap();
        (0..IDS_PER_THREAD)
            .map(|_| instance.generate_blocking())
            .collect()
    });

    let all_ids: Vec<_> = thread_results.into_iter().flatten().collect();
    assert_all_unique(&all_ids, "Concurrent max instance (31,31)");
    assert_components_match(&all_ids, WORKER_ID, PROCESS_ID, |id| {
        let c = id.components();
        (c.worker_id, c.process_id)
    });
}

#[test]
fn extreme_stress_16_threads() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    typedflake::id!(ExtremeStressId);

    const NUM_THREADS: usize = 16;
    const IDS_PER_THREAD: usize = 1000;
    const WORKER_ID: u64 = 5;
    const PROCESS_ID: u64 = 3;

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Clone Arcs for closure
    let success_clone = Arc::clone(&success_count);
    let error_clone = Arc::clone(&error_count);

    // Run extreme stress test with error counting
    let thread_results =
        run_concurrent_generation(NUM_THREADS, IDS_PER_THREAD, move |thread_idx| {
            let success = Arc::clone(&success_clone);
            let errors = Arc::clone(&error_clone);
            let instance = ExtremeStressId::instance(WORKER_ID, PROCESS_ID).unwrap();
            let mut ids = Vec::with_capacity(IDS_PER_THREAD);

            for _ in 0..IDS_PER_THREAD {
                match instance.generate() {
                    Ok(id) => {
                        ids.push(id);
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        ids.push(instance.generate_blocking());
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            // Verify thread-local uniqueness
            assert_all_unique(&ids, &format!("Thread {thread_idx} local uniqueness"));
            ids
        });

    // Verify global uniqueness and components
    let all_ids: Vec<_> = thread_results.into_iter().flatten().collect();
    assert_all_unique(&all_ids, "Extreme stress global uniqueness");
    assert_components_match(&all_ids, WORKER_ID, PROCESS_ID, |id| {
        let c = id.components();
        (c.worker_id, c.process_id)
    });

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    println!(
        "Extreme stress: {successes} successes, {errors} exhaustions across {NUM_THREADS} threads"
    );
    assert_eq!(successes + errors, NUM_THREADS * IDS_PER_THREAD);
}
