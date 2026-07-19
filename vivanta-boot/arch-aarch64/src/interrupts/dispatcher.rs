// ---------------------------------------------------------------------------
// IRQ dispatch table
// ---------------------------------------------------------------------------

use crate::exceptions::ExceptionFrame;
use super::gic;

const MAX_IRQ: usize = 256;

pub type IrqHandler = fn(u32);

static mut IRQ_TABLE: [Option<IrqHandler>; MAX_IRQ] = [None; MAX_IRQ];

pub unsafe fn register_irq(irq: u32, handler: IrqHandler) {
    let idx = irq as usize;
    if idx >= MAX_IRQ {
        panic!("register_irq: IRQ {} out of range (max {})", irq, MAX_IRQ);
    }
    if IRQ_TABLE[idx].is_some() {
        panic!("register_irq: IRQ {} already registered", irq);
    }
    IRQ_TABLE[idx] = Some(handler);
}

#[no_mangle]
pub unsafe extern "C" fn irq_entry_handler(
    frame: &mut ExceptionFrame,
    _kind: u64,
    _esr: u64,
    _far: u64,
) {
    let irq_id = gic::acknowledge();

    if irq_id != 0x3FF {
        let idx = irq_id as usize;
        if idx < MAX_IRQ {
            if let Some(handler) = IRQ_TABLE[idx] {
                handler(irq_id);
            }
        }
    }

    gic::eoi(irq_id);

    vivanta_arch_api::scheduler::scheduler_reschedule(frame as *mut ExceptionFrame as usize);
}
