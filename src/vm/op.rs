pub type ConstIndex = u8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    /// Load constant
    Constant(ConstIndex),
    Add,
    Subtract,
    Multiply,
    Divide,
    /// negate the sign of a number
    Negate,
    Return,
}

impl Op {
    /// Convert an op to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(3);
        match self {
            Self::Constant(const_val) => {
                bytes.push(0);
                bytes.push(*const_val);
            }
            Self::Add => bytes.push(1),
            Self::Subtract => bytes.push(2),
            Self::Multiply => bytes.push(3),
            Self::Divide => bytes.push(4),
            Self::Negate => bytes.push(5),
            Self::Return => bytes.push(6),
        }
        bytes
    }

    /// Construct a single op from the byte slice.
    ///
    /// If successful, will yield both the op and the number of bytes read
    pub fn read_from(bytes: &[u8]) -> Option<(Op, usize)> {
        let mut bytes_read = 1;
        let op = match bytes[0] {
            0 => {
                let const_index = bytes.get(1)?;
                bytes_read += 1;
                Op::Constant(*const_index)
            }
            1 => Op::Add,
            2 => Op::Subtract,
            3 => Op::Multiply,
            4 => Op::Divide,
            5 => Op::Negate,
            6 => Op::Return,
            _ => return None,
        };

        Some((op, bytes_read))
    }
}
