use std::heap::{Alloc, AllocErr, Heap, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem;
use std::intrinsics;

fn hash(mut x: u32) -> u32 {
    
    x = (x.wrapping_shr(16) ^ x).wrapping_mul(0x45d9f3b);
    x = (x.wrapping_shr(16) ^ x).wrapping_mul(0x45d9f3b);
    x = x.wrapping_shr(16) ^ x;
    return x;
}

pub struct IntegerHashTable {
    size: AtomicUsize,
    capacity: usize,
    array: *mut u32,
}

impl IntegerHashTable {
    pub fn new(capacity: usize) -> Result<Self, AllocErr> {
        assert!(capacity.is_power_of_two());

        // Multiply capacity by 2 to simulate the effect of a tuple of (u32, u32)
        let array_layout = IntegerHashTable::layout(capacity)?;
        let array_alloc = unsafe { mem::transmute(Heap.alloc_zeroed(array_layout)?) };

        Ok(IntegerHashTable {
            size: AtomicUsize::new(0),
            capacity,
            array: array_alloc,
        })
    }

    fn layout(capacity: usize) -> Result<Layout, AllocErr> {
        match Layout::array::<u32>(2 * capacity) {
            Some(layout) => Ok(layout),
            None => Err(AllocErr::invalid_input("Capacity overflowed layout.")),
        }
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::SeqCst)
    }

    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // FIXME(dvkelly) Not entirely sure what to do here as I would like to be 
    // able to share references across threads without syncronization
    // as the lock free nature is the whole point of the data structure.
    pub fn set(&self, key: u32, value: u32) -> Option<u32> {
        assert!(key > 0);
        assert!(value > 0);

        let mut idx = hash(key);
        loop {
            // Wrap array around to array length
            idx &= (self.capacity - 1) as u32;
            // Calculate pointer into array
            let key_addr = unsafe { self.array.add((idx * 2) as usize) };
            let value_addr = unsafe { self.array.add((idx * 2 + 1) as usize) };

            // Load key and value at location
            let probed_key = unsafe { intrinsics::atomic_load_relaxed(key_addr) };
            // If the keys don't match
            if probed_key != key {
                // Either the slot is filled
                if probed_key != 0 {
                    idx += 1;
                    continue;
                }

                // Or the slot can be filled with the data
                // FIXME(dvkelly) What is the purpose of the boolean returned by the intrinsic? And is it possible to use?
                let (previous_key, _) = unsafe {
                    intrinsics::atomic_cxchg_relaxed(key_addr, 0, key)
                };
                if (previous_key != 0) && (previous_key != key) {
                    // If the insertion doesn't succeed because of another thread,
                    // try another index
                    idx += 1;
                    continue;
                }

                // Only update size when filling empty slot
                self.size.fetch_add(1, Ordering::SeqCst);

                // If everything goes as planned, store new value in empty slot
                unsafe { intrinsics::atomic_store_relaxed(value_addr, value) };
                return None;
            }
            
            // If everything goes as planned, store new value and return old
            let old_value = unsafe { intrinsics::atomic_xchg_relaxed(value_addr, value) };
            return Some(old_value);
        }
    }

    pub fn get(&self, key: u32) -> Option<u32> {
        assert!(key > 0);

        let mut idx = hash(key);

        loop {
            // Truncate value to wrap around in array
            idx &= (self.capacity - 1) as u32;
            // Calculate pointer into array
            let key_addr = unsafe { self.array.add((2 * idx) as usize) };
            let value_addr = unsafe { self.array.add((2 * idx + 1) as usize) };

            // Load key and value relaxed
            let probed_key = unsafe { intrinsics::atomic_load_relaxed(key_addr) };

            if probed_key == key {
                // If key is present in map, return value
                let value = unsafe { intrinsics::atomic_load_relaxed(value_addr) };
                return Some(value);
            } else if probed_key == 0 {
                // If hash slot is empty, return None
                return None;
            }

            idx += 1;
        }
    }
}

impl Drop for IntegerHashTable {
    fn drop(&mut self) {
        match IntegerHashTable::layout(self.capacity) {
            Ok(layout) => unsafe {
                Heap.dealloc(mem::transmute(self.array), layout)
            }
            Err(_) => unreachable!()
        }
    }
}

unsafe impl Send for IntegerHashTable {}
unsafe impl Sync for IntegerHashTable {}

#[cfg(test)]
mod integer_hash_map_tests {

    use super::*;

    fn sync_only<T: Sync>(_: T) {}
    fn send_only<T: Send>(_: T) {}

    #[test]
    fn is_send() {
        send_only(IntegerHashTable::new(16));
    }

    #[test]
    fn is_sync() {
        sync_only(IntegerHashTable::new(16));
    }

    #[test]
    fn create_map() {
        let map = IntegerHashTable::new(128).unwrap();

        assert!(map.is_empty());
        assert_eq!(map.capacity(), 128);
    }

    #[test]
    fn insert_values() {
        let map = IntegerHashTable::new(128).unwrap();

        assert_eq!(map.set(10, 20), None);
        assert_eq!(map.set(20, 30), None);
        assert_eq!(map.set(30, 40), None);
        assert_eq!(map.set(40, 50), None);

        assert_eq!(map.size(), 4);
    }

    #[test]
    fn insert_retrieve() {
        let map = IntegerHashTable::new(128).unwrap();

        assert_eq!(map.set(10, 20), None);
        assert_eq!(map.set(20, 30), None);
        assert_eq!(map.set(30, 40), None);
        assert_eq!(map.set(40, 50), None);

        assert_eq!(map.size(), 4);

        assert_eq!(map.get(10).unwrap(), 20);
        assert_eq!(map.get(20).unwrap(), 30);
        assert_eq!(map.get(30).unwrap(), 40);
        assert_eq!(map.get(40).unwrap(), 50);
    }

    #[test]
    fn insert_update() {
        let map = IntegerHashTable::new(128).unwrap();

        assert_eq!(map.set(10, 20), None);
        assert_eq!(map.set(20, 30), None);
        assert_eq!(map.set(30, 40), None);
        assert_eq!(map.set(40, 50), None);

        assert_eq!(map.size(), 4);

        assert_eq!(map.set(10, 60), Some(20));
        assert_eq!(map.set(20, 70), Some(30));
        assert_eq!(map.set(30, 80), Some(40));
        assert_eq!(map.set(40, 90), Some(50));
    }

    use std::thread;
    use std::sync::Arc;

    #[test]
    fn multiple_thread_contention() {
        const NUM_THREADS: usize = 8;

        let mut handles = Vec::new();
        let map = Arc::new(IntegerHashTable::new(128).unwrap());

        for id in 1..(NUM_THREADS + 1) {
            let map = Arc::clone(&map);
            let handle = thread::spawn(move || {
                let my_id = id as u32;

                let mut values = Vec::new();
                for key in (1..11).map(move |v| v + 10 * my_id) {
                    map.set(key, my_id);
                    values.push(key);
                }

                values
            });

            handles.push((id, handle))
        }

        handles.into_iter().map(|(id, h)| (id, h.join().unwrap())).for_each(|(id, thread_keys)| {
            for key in thread_keys {
                assert_eq!(map.get(key), Some(id as u32));
            }
        });
    }
}
