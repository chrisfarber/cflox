# Source tracking

Right now, the AST is completely unaware of the source whence it was constructed.

Sure would be nice if this wasn't the case, so that friendly error messages could be reported. On the other hand, that is an endeavor in itself, and, it's not the primary one I set out to tackle in this prof dev project.

## Current context

The AST is currently based around expressions, but, statements are coming next. To recap, the current tree looks like:

### Tokens

Tokens are pretty flat. There's a struct with two fields:

- token type
- line

### AST

- Expression
  - Literal
    - Number
    - String
    - True
    - False (why did I make this its own thing?)
    - Nil
  - Unary
    - Negate
    - Not
  - Binary
    - which has left, right, and an op

(tangentially, I'm now wondering what the top of the AST is going to be once I actually do add statements. Perhaps a statement will be a special kind of expression?)

## Designing a tracking AST

The parser obviously has the tokens available as it constructs the AST. So, immediately I have some ideas:

### The naive

It could capture the most relevant token. Binary captures the binary op. Unary captures the ... unary. Literal captures the literal. What about expression or statement? Doesn't matter!

This will require the token to be updated to include a start (and end?) index.

### Actual spans

Right now we track lines, but, we could just as easily track indices into the source material. As Crafting Interpreters discusses, it's fine to recompute line + column from a single index; you only need to do this for each error.

To do this, we'd need to switch the tokens to capturing starting (and ending?) offsets within the source string. 

