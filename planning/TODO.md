# TODO

keeping track of things that are broken or still need to be done, outside of following subsequent chapters:

## Runtime errors don't carry a location

`LoxError` (`src/interpreter/error.rs`) has no span field, so runtime errors (undefined
variable, wrong arity, etc.) can't be rendered through `Diagnostic::render()` the way parse
and resolution errors are — they just print a bare message.

## Function AST nodes have inconsistent identity

Top-level `fun` declarations unwrap `Node<Function>` and discard its id
(`DeclarationKind::Function(Function)` in `parse_fun_declaration`), while class methods keep
the wrapper (`Class.methods: Vec<Node<Function>>`). Doesn't matter yet, but the resolver
already proved out the pattern the compiler will want next: a side table keyed by `NodeId`
(`Resolutions: HashMap<NodeId, u32>`). The moment the compiler wants to attach per-function
state (upvalues, a compiled chunk) the same way, top-level functions and methods will need
different code to find "this function's id." Normalize to always carry `Node<Function>`.

## Test Harness

I should build a suite of tests in lox code. This suite could then be run against both the interpreter and compiler in order to validate their operation. Right now I just have some files in `examples/` that I manually verify.

## Interpreter Function::bind deep-clones the whole method body

`value::Function.body` is `Box<Statement>`. `bind()` does `body: self.body.clone()` to attach
`this`, and since `Statement`/`Expression` are recursively `Clone`, every `obj.method()` call
deep-clones the entire method body's AST just to bind one variable.

## Small cleanups

- `try_binary` (`src/parser/mod.rs`) loops via `while let Ok(expr) = res`, but the only `Err`
  path already early-returns via `?` — it's really an `if`/loop hybrid in disguise. A plain
  loop tracking `expr` directly would be clearer, and reusing that shape is also the fix for
  the logical-operator-precedence bug above.
- `Node::convert` (`src/parser/node.rs`) is currently dead code (compiler warns on it). Either
  it's for an upcoming AST-lowering pass into the compiler (fine, just say so in a comment) or
  delete it for now.
- `Parser::current_span()`'s EOF fallback computes `pos = tok.span.start + tok.span.end`,
  which looks like it should just be `tok.span.end` (the position right after the last
  token). Currently harmless because `Diagnostic::render()` clamps both span ends to the
  source length regardless of how far out of range they are, but worth fixing for
  correctness's sake since a future consumer of spans might not go through that clamp.
