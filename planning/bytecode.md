# Bytecode

To start building out the VM, the book starts with its bytecode and chunks.

Right away, I have some choices to make when doing this in Rust.

- Do I make Operations `repr(u8)`? Do I bring in num_enum?
- Do I associate values with Opcodes?
