Composing simulataneous changes
===============================

How can we compose simulataneous changes?

Types of changes considered:
- rename variables
- tweak expressions not involving control flow
- tweak expressions involving control flow
- change control flow of a function
- fix crashes
- factorize code
- remove function
- add function
- tweak and add function call
- swap instructions / branches

Some changes are semantically preserving program, like renaming, factorize or
changing function set on functions called nowhere.

Semantic preserving changes compose well if they do not intersect. Definition
of intersection is not hard but might be quadratic on number of expressions.

Quadratic work in number of type of changes required?
Can we replace that with a change priorization system?
Is there a systematic way of describing compatible changes?

Idea:
For $i \in \{1,2\}$, if $\Delta_i$, is a difference of the form 
$\delta_{i,1}, \ldots, \delta_{i,n}$.
Then $\Delta_1 & \Delta_2$ is well defined if any interleaving
of $\delta_{i,n}$ is equivalent.
=> Needs refinement: see `shadowing.imp`.

Changing control flow
---------------------

How to handle changes that modify the control flow of a function ?
See `branches.imp`.

Functions
---------

See `functions.imp` and `funsig.imp`.
What happens if someone modifies a function ?
Are all usages considered modified ?
Then most modifications will collide with each other.

Other approach: Functions always embody a notion of implicit specification.
Modifiying a function is not "modifying" callers.
Caveats: Can create crashes, see `functions.imp`.

Retained approach for as much flexibility as possible:
Annotate each change with a "spec preservation" flag.

What happens if someone modifies function arguments?
- Generalization: new argument with one value giving the same behaviour as before
- Specialization: remove argument, every call must have had the same value for it
- Change: as if the function was deleted and recreated
