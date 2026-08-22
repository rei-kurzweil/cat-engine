# MMS component lifecycle

MMS uses these terms consistently:

| Operation | Result |
| --- | --- |
| parse source | `ComponentExpression` AST |
| materialize a component AST | `MaterializedCE` deferred template |
| evaluate MMS code | runtime `Value` |
| instantiate a template | detached live `ComponentObject` / `ComponentId` |
| attach | add an existing live component below a parent |
| initialize | run the live component-tree init walk |

`MaterializedCE` is neither an AST node nor a live ECS object. An ordinary
module import evaluates its module once per session identity. Direct component
exports are instantiated once as detached live objects, so repeated imports
share the same handle. A function export remains a function; every call that
returns a component creates a fresh live object. `import ast` is the only
author-facing syntax for importing a deferred component template and has no
registration, attachment, or initialization side effects.
