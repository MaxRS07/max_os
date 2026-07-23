use crate::syscall::SyscallArgs;

pub enum ComOp {
    Pipe,
    Socket,
    Shmget,
    Semget,
    Msgget,
}

impl Communication {
    pub fn call(&self, args: SyscallArgs) {
        match self {
            Self::pipe => 
        }
    }
}

impl From<usize> for Communication {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::pipe,
            1 => Self::socket,
            2 => Self::msgget,
            4 => Self::semget,
            5 => Self::msgget
        }
    }
}

fn pipe(arg) {}
