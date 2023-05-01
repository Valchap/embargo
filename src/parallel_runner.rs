use std::{
    sync::{Arc, Mutex},
    thread,
};

pub fn parallel_run<T, U>(data: U, function: fn(U::Item) -> T) -> Vec<T>
where
    T: Send + 'static,
    U: IntoIterator,
    U::IntoIter: Send + 'static,
    U::Item: 'static,
{
    let iterator = Arc::new(Mutex::new(data.into_iter()));

    let out = Arc::new(Mutex::new(Vec::new()));

    let thread_count = thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);

    let mut handles = Vec::with_capacity(thread_count);

    for _ in 0..thread_count {
        let iterator_clone = iterator.clone();
        let out_clone = out.clone();

        handles.push(thread::spawn(move || loop {
            let next = iterator_clone.lock().unwrap().next();

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

    Arc::try_unwrap(out).map_or_else(|_| unreachable!(), |mutex| mutex.into_inner().unwrap())
}
