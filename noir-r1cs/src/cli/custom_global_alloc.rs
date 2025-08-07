use std::{
    alloc::{GlobalAlloc, Layout},
};
use std::fs::File;
use std::sync::Mutex;
use memmap2::MmapMut;
use tempfile::tempfile;
use tracing::instrument;
use crate::measuring_alloc::MeasuringAllocator;

pub struct MemMap2File {
    file: File,
    mmap: MmapMut,
}

impl MemMap2File {
    pub fn new(size: usize) -> Self {
        let file = tempfile().expect("file created");
        file.set_len(size as u64).expect("file resized");

        let mmap = unsafe { MmapMut::map_mut(&file).expect("mmap mut created") };

        Self {
            file,
            mmap,
        }
    }
}

pub struct CustomGlobalAllocator {
    allocations: Mutex<Vec<MemMap2File>>,
    parent: MeasuringAllocator,
}

impl CustomGlobalAllocator {
    pub const fn new(parent: MeasuringAllocator) -> Self {
        Self {
            allocations: Mutex::new(Vec::new()),
            parent,
        }
    }
}

const SIZE_16MB: usize = 16 * 1024 * 1024;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CustomGlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() > SIZE_16MB {
            let mut allocations = self.allocations.lock().unwrap();

            let mut map = MemMap2File::new(layout.size());
            let ptr = map.mmap.as_mut_ptr();
            allocations.push(map);

            ptr
        } else {
            self.parent.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() > SIZE_16MB {
            let mut allocations = self.allocations.lock().unwrap();

            for (pos, map) in allocations.iter().enumerate() {
                if map.mmap.as_ptr() == ptr {
                    allocations.remove(pos);
                    return;
                }
            }

            panic!("Could not find memory to deallocate.")
        } else {
            self.parent.dealloc(ptr, layout)
        }
    }
}
