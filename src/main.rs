use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    sync::Mutex
};

use task::{Task, TaskType};

use futures::future::join_all;

#[tokio::main]
async fn main() {
    let (seed, starting_height, max_children) = get_args();

    eprintln!(
        "Using seed {}, starting height {}, max. children {}",
        seed, starting_height, max_children
    );

    let mut count_map = HashMap::new();
    let taskq = Arc::new(Mutex::new(VecDeque::from(Task::generate_initial(seed, starting_height, max_children))));
    
    let output = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    while Arc::strong_count(&taskq) > 1 || !(taskq.lock().unwrap().is_empty()){
        let curr_task;
        {
            //taskq may be empty
            if taskq.lock().unwrap().is_empty() {
                continue;
            } else {
                curr_task = (*taskq.lock().unwrap()).pop_front();
            }
        }

        let next = curr_task.unwrap();

        *count_map.entry(next.typ).or_insert(0usize) += 1;
        let output_clone = output.clone();
        let taskq_clone = taskq.clone();
        let handle = tokio::spawn(async move {
            let result = next.execute();
            {
                taskq_clone.lock().unwrap().extend(result.1.into_iter());
            }
            output_clone.fetch_xor(result.0, Ordering::SeqCst);
        });
        std::mem::drop(handle)
    }
    
    let end = Instant::now();

    eprintln!("Completed in {} s", (end - start).as_secs_f64());

    println!(
        "{},{},{},{}",
        output.load(Ordering::SeqCst),
        count_map.get(&TaskType::Hash).unwrap_or(&0),
        count_map.get(&TaskType::Derive).unwrap_or(&0),
        count_map.get(&TaskType::Random).unwrap_or(&0)
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
