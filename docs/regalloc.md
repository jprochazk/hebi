## Register allocator

We use a modification of the [linear scan](https://web.archive.org/web/20221205135642/http://web.cs.ucla.edu/~palsberg/course/cs132/linearscan.pdf)
algorithm with infinite registers. This means we do not perform register
spills. Register allocation is local to each function.

In summary, the algorithm works by tracking the liveness intervals of
registers, and scanning the intervals to determine when registers may be
reused.

It works in two phases:
1. Liveness analysis
2. Allocation

### Liveness analysis

During liveness analysis, registers are allocated for variables and
intermediate values used in expressions, and each usage of active registers
is tracked.

### Allocation

During allocation, the live intervals are traversed for the purpose of
constructing an index mapping each register to its final slot.
This mapping is done on a first-fit basis. The final slot is the first
free register at the time when the register was allocated.

### Example

After tracking, the live intervals are:

```text,ignore
 a │ ●━━━━━●
 b │    ●━━━━━━━━━━━━━━━━━━━━━━━●
 c │          ●━━━━━━━━━━━━━━●
 d │             ●━━━━━━━━●
 e │                ●━━●
   ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  0  1  2  3  4  5  6  7  8  9  10
```

Given the live intervals above, the scan would proceed step-by-step as
follows:

0. a=r0 <- no free registers, `r0` is allocated
1. a=r0, b=r1 <- no free registers, `r1` is allocated
2. a=r0, b=r1 <- `r0` is freed
3. b=r1, c=r0 <- `r0` is reused
4. b=r1, c=r0, d=r2 <- no free registers, `r2` is allocated
5. b=r1, c=r0, d=r2, e=r3 <- no free registers, `r3` is allocated
6. b=r1, c=r0, d=r2, e=r3 <- `r3` is freed
7. b=r1, c=r0, d=r2 <- `r2` is freed
8. b=r1, c=r0, <- `r0` is freed
9. b=r1 <- `r1` is freed
10. done

Each `<name>=<register>` pair is a mapping from a tracking register to a
final register. This information is then used to patch the bytecode.

The maximum register index at any point was `3`, meaning this function will
need 4 registers.
