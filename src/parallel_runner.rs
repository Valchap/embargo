use std::{
    sync::{Arc, Mutex},
    thread,
};

pub fn parallel_run<I, O>(data: Vec<I>, function: fn(I) -> O) -> Vec<O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    let iterator = Arc::new(Mutex::new(data.into_iter()));

    let out = Arc::new(Mutex::new(Vec::new()));

    let thread_count = if let Ok(count) = thread::available_parallelism() {
        count.get()
    } else {
        1
    };

    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let iterator_clone = iterator.clone();
        let out_clone = out.clone();

        handles.push(thread::spawn(move || loop {
            let next = {
                let mut iterator_mutex = iterator_clone.lock().unwrap();
                iterator_mutex.next()
            };

            if let Some(value) = next {
                let result = function(value);

                out_clone.lock().unwrap().push(result);
            } else {
                break;
            }
        }));
    }

    for t in handles {
        t.join().unwrap();
    }

    if let Ok(arc_content) = Arc::try_unwrap(out) {
        arc_content.into_inner().unwrap()
    } else {
        unreachable!();
    }
}
