# Garbage Collection

In chapter 8, the book introduces the concepts of environment and scopes. Environments are essentially just hashmaps, while scopes are nestable collections of environments. A scope can inherit from its parent scopes (recursively). New variables defined in a scope are discarded when the scope is discarded.

Scopes in lox are lexical, which means their lifetimes are tied to the structure of the language. For now, it's tempting to just treat scopes as a stack, since we are tree-walking our AST.

This approach would fall apart once we get to functions:

A function's scope does not inherit from the scope in which it's _called_. It inherits from the scope in which it is _defined_.

So, to properly implement this, I need:

1. A reference counting or garbage collection scheme. Maybe. I'm thinking about closures specifically; without closures, you have functions that inherit from the global environment. Actually, this reinforces my point: there can be many scopes active that inherit from common ancestor scopes.
2. A way for AST nodes to be shared? This may just actually be clone(). At any rate, a function Value will need to close over the AST nodes that comprise its body, so that it can tree-walk-evaluate them.

The problem is that I'm doing this all in Rust, and need to make the ownership model work. I'm not certain if I'll need a full fledged GC scheme, or something pared back. I'm also not sure if I should implement this myself as a learning exercise, or just grab one of the crates available in the ecosystem and run with it. After all, I did come here to write an interpreter, not dive into potentially unsafe rust.

For now, probably the simplest thing I could do is see how far `Rc<RefCell<Environment>>` will get me.

## Avoiding RefCell

I don't like that the Rc + RefCell approach turns this into runtime borrow checks. It also seems to be fighting rust. I could make it work, but, it feels like I am depriving myself of a learning opportunity.

So, that leaves a heap-like structure.

This could be as simple as `Vec<T>` and `usize` pointers. Not quite perfect though: I'd need to keep my own free list. If I wanted to be fancy.

It would enforce that there's only one mutable borrow … to the entire heap. In practical terms, I think this means an environment would need to have all of its methods accept the heap as an argument.

```rs
impl Environment {
    pub fn get(&self, heap: &Heap<Self>, ident: String) -> &Value;
    pub fn define(&mut self, heap: &Heap<Self>, ident: String, value: Value);
}
```

Values for the most part are literals. But they can be references to functions or classes. So, therefore, values need to become... pointers?

```rs
pub enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Pointer(usize),
    // or, break out the type separately:
    // Function(usize),
    // Class(usize),
}

impl Value {
    // not all methods would need to reference a heap, actually. equals could use pointer equality.
    // same for is_truthy, get_number(), etc.
    // But, say, get_function() would need the heap so it could look it up and borrow it.

}
```

I suppose this approach raises an explicitly interesting design decision: Should the interpreter keep a Heap of an Enum type with a case for all things that could be in the heap (Function, Class, etc), or should the interpreter keep one heap per type?

### "Arenas"

The approach above is known as as using arenas. Naturally, there are even plenty of crates out there that implement it, so I could easily grab one of those instead of rolling my own.

The benefit of this approach is that you don't fight the borrow checker and still benefit from its rules. You need a mutable borrow to the heap to make changes. The "pointers" and their access are still checked by rust. You can't access uninitialized memory and run with it. Depending on the arena, you could potentially get mixed up and retrieve a _different_ object than you intended to, though.

I can think of a few downsides to using arenas, however:

1. Pointers are more expensive. Each access has to indirect through the arena (probably a vec underlying it), so you dereference twice.
2. Lots of arenas? It's probably better to use one area per type of object, because it's more memory efficient (no need to reserve space to tag each item). Honestly I'm not sure if this is true: when I imagine a GC, doesn't each object need a type? If it is true, then, passing them around to functions will get annoying.
3. Rust's ownership model is still at play and will influence the design of my interpreter. I feel like this _could_ become a problem, although I can't quite see how at the moment.

## Garbage Collection

My last option, or, perhaps, pair of options, is to reach for garbage collection.

Given that lox is fundamentally a garbage-collected language, this could actually be the best approach. It's going to make implementing the interpreter much easier, since I won't be continually wondering how to design/architect my code so that it works _with_ the borrow checker — my background in C, Lisp, etc., has provided me with experience on how to write good software, and I can leverage that without the borrow-checker side quest. (The flip side: is this depriving me of learning how to effectively write rust? That was, after all, part of my motivation of using rust here).

If I decide to go this route, I see two good options:

1. Grab a garbage collector from crates.io.
2. Write my own.

### Writing my own

Writing my own would be quite a challenge. But it also could be the right move. I've been working through the first part of the book in Rust, and haven't yet made a decision on what to do about the second part. Instead of Java, the book has you use C for the second part, which means you are truly responsible for everything, even GC, as there's no runtime to inherit one from.

In other words, if I keep going like I hope to, I'm eventually writing a garbage collector anyway. And if I keep going in rust, then, why _not_ scaffold one now?

I haven't made a decision about Rust vs C for the second part, though. I've written plenty of C and ObjC in the past, picking it up again would be no problem.

Regardless, if I proceed with writing my own GC in Rust, I wouldn't implement the collector yet. I'd design an interface that _looks_ like a GC, but just never actually frees any memory.

Perhaps this would require some use of unsafe and rust pointers? That may be valuable experience even if I don't write the collector in Rust.

## Decision

As much as I'd like to continue following the second part of the book in Rust, I see some obstacles.

- Looking at [this blog post about rust-gc](https://manishearth.github.io/blog/2015/09/01/designing-a-gc-in-rust/), I can tell there are many design/engineering problems with making GC work in Rust. It's essentially a project in its own.
- I see topics such as "tagged unions" and "dynamically typed numbers" in the ToC that make me presume that using Rust will be incongruent with the lessons the book is teaching.

So, I ask myself, what will I learn from continuing with the second part in rust?

I'm really weighing "what will I learn about writing rust" against the ability to make more rapid progress through the book, and somewhat against some of the lower-level tricks it will teach.

Given that, the plan I'm arriving at is, roughly:

- Just use Rc<RefCell<T>>. The runtime cost is inconsequential for this purpose. It will let me make progress without diving into the incredibly hairy world of garbage collection in Rust. I don't care if cycles arise and leak memory.
- I can defer my decision of whether to go C or Rust for part 2
