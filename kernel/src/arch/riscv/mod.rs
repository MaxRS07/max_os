pub mod mtvec;

pub fn setup() {
    mtvec::init_trap_handler();
}
