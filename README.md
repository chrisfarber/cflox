# Chris's Lox Interpreter

I'm working through the book Crafting Interpreters, by Robert Nystrom.

This repository contains my interpreter for Lox, the programming language used in the book.

I decided to write my interpreter in Rust, instead of Java as the book suggests, because:

- Life's too short for me to write any more Java
- I didn't feel like installing the JVM again
- I've enjoyed my experience with Rust and would like to become more proficient with it
- I just really don't enjoy OOP
- But, I guess I do enjoy making things harder for myself

So far, this choice has seemed totally fine. The most interesting part has been the ability to skip implementing a large amount of generated code; on the flip side, I did really enjoy the book's discussion of comparing the natures of FP and OO languages. That was the first time the visitor pattern has made sense to me.

## LLM use

This project is primarily a learning exercise, and, as such, I have hand-written almost all code present here. I have found Claude helpful when it comes to questions about idiomatic patterns in rust, and I have occasionally asked it to find bugs (it has). I also had no interest in writing the code that pretty-prints diagnostic messages with their related source code, so I had Claude do this. All of the work of defining my AST structures and tracking spans was done by me.
