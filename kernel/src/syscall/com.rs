use crate::syscall::SyscallArgs;

pub enum ComOp {
    Pipe,
    Socket,
    Shmget,
    Semget,
    Msgget,
}

impl ComOp {
    pub fn call(&self, args: SyscallArgs) {
        match self {
            _ => todo!(),
        }
    }
}

impl From<usize> for ComOp {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Pipe,
            1 => Self::Socket,
            2 => Self::Msgget,
            4 => Self::Semget,
            5 => Self::Msgget,
            _ => panic!("Unsupported communication operation"),
        }
    }
}

fn pipe(arg: usize) {}
