use seloria_core::Block;
use seloria_vm::ExecutionResult;

pub trait BlockEventSink: Send + Sync {
    fn on_block_committed(&self, block: &Block, results: &[ExecutionResult]);
}
