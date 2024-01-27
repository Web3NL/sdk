pub mod config;
pub mod fs;
pub mod heap;

use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};
use std::cell::RefCell;

static MEM_ID_CONFIG: MemoryId = MemoryId::new(0);
static MEM_ID_FILE_DATA: MemoryId = MemoryId::new(1);
static MEM_ID_METADATA: MemoryId = MemoryId::new(2);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

struct MemoryManagerStore;

impl MemoryManagerStore {
    fn get(id: MemoryId) -> VirtualMemory<DefaultMemoryImpl> {
        MEMORY_MANAGER.with(|mm| mm.borrow().get(id))
    }
}
