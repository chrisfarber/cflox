# TODO

keeping track of things that are broken or still need to be done, outside of following subsequent chapters:

## Logical Operator precedence

It's broken.

```
> 1 or 2 and 3 or 4;
parse error: Diagnostic {
    severity: Error,
    span: Span {
        start: 13,
        end: 15,
    },
    message: "expected token Semicolon, found Or",
}
```

## Spans

In the lexer, I think I should be resetting the cur_token_start in push_token

## Diagnostic reporting

I have spans, and maybe they are roughly correct. I should implement reporting for diagnostics: show the line, with squigglies underneath, and the diagnostic message

## Runtime errors

Two things:

1. report line numbers via runtime errors when possible
2. Handle runtime errors in the interpreter correctly. Report correct return codes to the OS.

## Number lexing

`42.` is parsed as a number consuming the period. When there's no trailing digit, I think lox actually doesn't consume the dot, in order to allow for `42.foo`

## Spans when creating a Block from Vec<Declaration>

It currently uses the start of the first element and end of the last element. It's probably better to fold while min/maxing each element's spans.
