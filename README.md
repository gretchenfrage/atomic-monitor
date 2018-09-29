
# Atomic Monitor

The atomic monitor is a utility that behaves like a monitor over atomic data, 
and has the potential to be more performant in certain circumstances.

A monitor is a useful utility for one thread to block on some data to fulfill 
a condition, and another thread to mutate that data to satisfy that condition.
However, every mutation to the data involves the acquisition of a mutex, which 
can be unnecessarily slow, especially when mutations are simple operations which 
could be performed atomically.

The atomic monitor provides a monitor-like utility, for which lock acquisition 
is only performed when necessary. That is, only when one thread is actually blocking 
on a condition -- not if the data is only being atomically mutated, and not when
a thread blocks on a condition which is already satisfied.

### Example

    use atomicmonitor::AtomMonitor;
    use atomicmonitor::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;

    // atom monitor around an integer
    let atom_monitor = Arc::new(AtomMonitor::new(0u32));

    // 100 threads which increment the integer 10 times
    for _ in 0..100 {
        let atom_monitor = atom_monitor.clone();
        thread::spawn(move || {
            for _ in 0..10 {
                atom_monitor.mutate(|atomic_int| {
                    atomic_int.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
    }

    // wait until the integer count reaches 1000
    atom_monitor.wait_until(|int| int == 1000);

### Implementation

The `AtomMonitor` contains three data, the atomic data being guarded, an atomic 
*notification request counter*, and a regular monitor which guards `()`. 


Upon atomic mutation to the data, the request counter is loaded, and only if the 
number is positive, the monitor is acquired and notified. 

Upon awaiting a certain condition on the data, the data is atomically loaded,
and only if the condition is not already satisfied, the request counter is 
atomically incremented, the monitor is blocked on until the condition is fulfilled, 
and then the request counter is atomically decremented on exit.

