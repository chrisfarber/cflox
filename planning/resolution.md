# Resolutions

The interpreter needs to be changed to use the resolutions.

This happens in a few places:

- `evaluate()` ExpressionKind of Assign
- `evaluate()` ExpressionKind of Variable

When we define it's _always_ in the current scope. This is, for example, how function calls work when binding arguments.

So, importantly, we only need to track distances on Expression nodes. It's strange that we have to associate this data with an expression node since not all expression nodes really need the distance.
