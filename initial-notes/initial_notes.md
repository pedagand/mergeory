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

Objective: Avoid quadratic human work
=====================================

Take inspiration from Darcs or Pijul
------------------------------------

Both are using a category approach to merge problems by defining what is a patch.

Darcs: Checks that $A^{-1}B$ and $BA^{-1}$ commute.

Pijul: replaces files by "graggles" in its internal states (graphe of file lines)
Idea: one should push changes as backward as possible "antiquing"
-> make sequential changes parallel whenever possible

Not sure anything relevant is to be taken here.

Simplify: Only refactorings
---------------------------

This shows that for comparing refactorings, the current definition on "tactics"
of the parallel composition "&" is too weak.
For example, if the oracles are just checking similar behaviours on the set of
states, they will accept the no-op as a valid merge of two complex refactoring
operations.

We shoud be able to capture the intent beneath the refactoring operation.
But "tactic" like language is requiring a quadratic work even just for refactoring.

May we use a notion of refactoring scale?
I think that intent matters more than scale.

Simplify: Only subexpressions
-----------------------------

Here it is very important to care about modifications that are spec preserving
and other modifications.

I think that we can encode a non-spec preserving modification as adding a new
function, replacing all calls and then removing the original, and then say that
all functions are black boxes.
Anyway, two concurrent spec changing modifications on the same function should
be taken with care and not merged without human checking.

If all the changes we are interested in are subexpressions replacements, then,
in the case where no control flow are impacted, it is easy to deal with by
taking the ad-hoc ValueChange oracle in Girka's thesis encoding.
Then we can try to compose the changes. If a value is in both change set, then
there is no good fusion.
Actually we can accept a bit more modifications by taking control flow into
account in ValueChange oracle, to allow the same variable to be modified
concurrently in separated branches.

We can also theoretically accept programs that modify the control flow easily
with oracles that looks like ValueChange but merges after the executions of
modified branches.

### More formally: A generalized ValueChange correlation oracle
Static evaluation environment = Set of non-spec preserved functions
Dynamic evaluation environment = States of program 1 * States of program 2 * modified variables

Invariant =
```
I(mf, ((k1, S1), (k2, S2), mx) =
    dom(S1) = dom(S2) && forall x not in mx, S1(x) = S2(x)
```

Interpretation function = in very high level pseudo-code
```
match k1, k2 with
    | skip, skip
    | sequence _ _, sequence _ _
    | if (c) _ else _, if (c) _ else _ when c not in mx or mf
    | while (g) _, while (g) _ when g not in mx or mf
    | assert (b), assert (b) when b not in mx or mf
    | halt, halt ->
        progress by one in each program, do not change mx
    | x = e, x = e when e does not contain anything in mx or mf ->
        progress by one in each program, remove x from mx
    | x = e1, x = e2 -> (* For our usecase do not treat divisors separately *)
        progress by one in each program, add x to mx
    | if (c1) et1 else ee1, if (c2) et2 else ee2 ->
        progress until exiting the if branch in both,
        add all encoutered variables in both branches in both programs to mx
    | while (g1) e1, while (g2) e2 ->
        progress until exiting the loop,
        add all encoutered variables in both programs to mx
```

Maybe this kind of oracle is too simple. It maintains a list of which variables
have changed but not to what they changed.
But it does not matter for refusing semantically colliding patches.

If we think this is a right approach, I should detail more what happens in case
of non termination.

### A oracle on oracle to check absence of collisions

To check if merging two distinct ValueChange oracle correspond to modifications
that could be merged, we can use a higher level oracle that interleaves the two
ValueChange oracles and check that at any point if a variable is modified, it is
not the case in the other change.
This can use the common states in the source program to merge the paths.
Moreover, this kind of oracles, could be used to specifically show the points
where human attention should be focussed, whenever they cannot be constructed.

If we take this path, I will make a more precise oracle on oracle description.

In Coq proofs, Thibault Girka says that oracles on oracles are difficult to make
because equality is not decidable on oracles. Will this be a problem?

### Caveats

In real life, I feel that concurrently modified programs will always share some
variables in their change set, except for dead code modifications, or
modifications on distinct branches.
This might really generate a lot of conflicts.

Generalize back
---------------

Correlate oracles with oracles might be possible, but it seems ad-hoc for
programs that have comparable structure.
I think that we will have problems whenever the states correlated by distinct
oracles start to diverge in arbitrary directions.
There is no reason for different points to merge again after some time when we
remove the program shape preservation.

Still, restricting ourselves to specific kind of correlation oracles seems
necessary.
Then what could be a good expressive enough correlation oracle family?

What about addition / deletion of control struture.
Branch modifications seems to be manageable in the same way as changing values.
What about loops?

Conflict resolutions
====================

How can conflicts be solved if there is any and how to represent a conflict?

ExprChange: Another representation for ValueChange-like oracles
---------------------------------------------------------------
Change point of view: We do not use two programs as input but a single one
and a potential modification for each original expression.

We denote by `new(e)` the modified version of `e` by the list of modifications.

Static evaluation environment = Set of non-spec preserved functions * list of modifications
Dynamic evaluation environment = States of original program * Store of the modified program * modified variables

In case of crash, replace the corresponding store by \bottom. If both stores
are crashed, crash the correlation as well.

Invariant = `I(mf, ((k, S), new_S, mx)) = forall x not in mx, S(x) = new_S(x)`

Interpretation function = in very high level pseudo-code
```
match k with
    | skip
    | sequence _ _
    | halt ->
        progress by one instruction, do not change mx
    | if (c) _ else _
    | while (c) _
    | assert (c) ->
        if (c preserved by new and does not contain anything in mx or mf):
            go to the next step, do not change mx
        else:
            progress until exiting the branch, add all assigned variables to mx
    | x = e ->
        if (e is preserved by new and does not contain anything in mx or mf):
            progress by one instruction, remove x from mx
        else:
            progress by one instruction, add x to mx
```

This new representation shows the fact that a program not modifying structure
but maybe control flow can be tracked with only keeping the states of the source
program.

Merge two ExprChange modifications
----------------------------------
The description of ExprChange make it easy to derive a correlation corresponding
to two changes in parallel by fusing the lists of modifications as the modified
programs are now implicit.

However, doing so might create conflicts when an expression is modified in both
or if a modified expression uses a variable also modified in the other program.

Therefore a merging of two concurrent change is described with three sets of
expression modifications.
Two of them denoted by `left(e)` and `right(e)` are the merged modifications and
play symmetrical roles.
A third one `override(e)` is used for solving conflicts between the
modifications.
If `dom(override) is empty`, then the two sets of modifications are independant
and can be easily combined without manual input.
However, some concurrent modification will require an override expression that
must be given by the user.

To check that a program is indeed a merge of two ExprChange, we can use an
oracle MergeExprChange similar to ExprChange but with more parameters.
It will ensure that all conflicts are solved by manual overriding (and can
detect a potential missing override entry).


Static evaluation environment = Set of non-spec preserved functions * list of modifications (left, right and override)
Dynamic evaluation environment = States of original program * Store of the left program * Store of the right program * Store of the merged program * Set of touched variables (lx, rx and ox)

Where partial stores only contain values that differ from the original program.

In case of crash, replace the corresponding store by \bottom. If all stores
are crashed, crash the correlation as well.

Invariant = ```
I(mf, ((k, S), S_left, S_right, S_merged, (lx, rx, ox))) =
    forall x not in lx, S(x) = S_left(x),
    forall x not in rx, S(x) = S_right(x),
    forall x not in lx \cup rx \cup ox, S(x) = S_merged(x),
    lx \cap rx \subseteq ox
```

Interpretation function = in very high level pseudo-code
```
enum ModifStatus { UNCHANGED, OVERRIDEN, LEFT, RIGHT, CONFLICT }
fn expr_modif(expr) {
    if expr in override:
        return OVERRIDEN
    if expr in left:
        if expr in right:
            return CONFLICT
        for litt in left(expr):
            if litt in rx and not in ox:
                return CONFLICT
        return LEFT
    if expr in right:
        for litt in right(expr):
            if litt in lx and not in ox:
                return CONFLICT
        return RIGHT

    # The expression itself is not modified but it might depend on modified
    variables
    flags = {}
    for litt in expr:
        if litt in ox:
            flags += OVERRIDEN
        else:
            if litt in lx:
                flags += LEFT
            if litt in rx:
                flags += RIGHT

    match flags with
    | {} -> UNCHANGED
    | {LEFT, RIGHT, ...} -> CONFLICT
    | {LEFT, ...} -> LEFT
    | {RIGHT, ...} -> RIGHT
    | {OVERRIDEN} -> OVERRIDEN
}

match k with
    | skip
    | sequence _ _
    | halt ->
        progress by one instruction, do not change lx, rx or ox
    | if (c) _ else _
    | while (c) _
    | assert (c) ->
        match expr_modif(c) with
        | NONE ->
            go to the next step, do not change lx, rx or ox
        | OVERRIDEN ->
            for expr_inside in branches:
                match expr_modif(expr_inside) with
                | NONE | OVERRIDEN -> pass
                | _ -> assert false
            progress until end of block and add all modified variables to ox
        | LEFT ->
            for expr_inside in branches:
                match expr_modif(expr_inside) with
                | RIGHT | CONFLICT -> assert false
                | _ -> pass
            progress until end of block and add all modified variables to lx
        | RIGHT -> (symmetrical from left...)
        | CONFLICT -> assert false
    | x = e ->
        match expr_modif(e) with
        | NONE ->
            go to the next step, remove x from lx, rx and ox
        | OVERRIDEN ->
            go to the next step, remove x from lx and rx, add x to ox
        | LEFT ->
            go to the next step, remove x from rx and ox, add x to lx
        | RIGHT ->
            go to the next step, remove x from lx and ox, add x to rx
        | CONFLICT -> assert false
```

The following case must be considered seriously:
```
let a = modified by left;
let b = modified by right;
let o = [overriden] a + b;
let c = a + o;
let d = c + b;

a         b
|         |
+···>o<···+
|    |    |
|    v    |
+--->c    |
     |    |
     v    |
     d<---+
```
We should consider that the variable c is modified by the left change but also
partly overriden.
Indeed, it will have a value different from just applying the left patch, but
it should also collide with d.
