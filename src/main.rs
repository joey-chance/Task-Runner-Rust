use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    sync::Mutex
};

use task::{Task, TaskType};

// use futures::future::join_all;

#[tokio::main]
async fn main() {
    let (seed, starting_height, max_children) = get_args();

    eprintln!(
        "Using seed {}, starting height {}, max. children {}",
        seed, starting_height, max_children
    );

    // protect both with a mutex
    // let mut count_map = HashMap::new();
    let count_map = Arc::new(Mutex::new(HashMap::new()));
    let taskq = Arc::new(Mutex::new(VecDeque::from(Task::generate_initial(seed, starting_height, max_children))));

    // output can be safely accessed from multiple threads
    let output = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    while let Some(next) = taskq.lock().unwrap().pop_front() {
        // process initial set of tasks
        // wrap it in a tokio task
        // new tasks won't be added to the taskq but instead the handle of the task will be saved in another vector
        let count_map = count_map.clone();
        let output = output.clone();
        let taskq = taskq.clone();
        let mut pending_tasks = Vec::new();
        let handle = tokio::spawn(async move {
            *count_map.lock().unwrap().entry(next.typ).or_insert(0usize) += 1;
            let result = async {next.execute()}.await;
            output.fetch_xor(result.0, Ordering::SeqCst); // DONE
            taskq.lock().unwrap().extend(result.1.into_iter());
        });
        pending_tasks.push(handle);
    }
    let end = Instant::now();

    eprintln!("Completed in {} s", (end - start).as_secs_f64());

    println!(
        "{},{},{},{}",
        output.load(Ordering::Relaxed),
        count_map.lock().unwrap().get(&TaskType::Hash).unwrap_or(&0),
        count_map.lock().unwrap().get(&TaskType::Derive).unwrap_or(&0),
        count_map.lock().unwrap().get(&TaskType::Random).unwrap_or(&0)
    );
}

// There should be no need to modify anything below

fn get_args() -> (u64, usize, usize) {
    let mut args = std::env::args().skip(1);
    (
        args.next()
            .map(|a| a.parse().expect("invalid u64 for seed"))
            .unwrap_or_else(|| rand::Rng::gen(&mut rand::thread_rng())),
        args.next()
            .map(|a| a.parse().expect("invalid usize for starting_height"))
            .unwrap_or(5),
        args.next()
            .map(|a| a.parse().expect("invalid u64 for seed"))
            .unwrap_or(5),
    )
}

mod task;
